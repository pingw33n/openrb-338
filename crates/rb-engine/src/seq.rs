//! The sequencer: the song position, the track events, the chase, the loop and the transport
//! (`docs/initial_investigation.md` section 5).
//!
//! The DLL's sequencer object sits at `song+8` (ctor `sub_1001EC50`): the position as a double at
//! `+0`, the last tick at `+8`, the song-mode tick saved by pattern mode at `+0x10`, the track
//! count at `+0xC` and the 32 track pointers at `+0x14` (nine are used, one per device; their
//! back pointer to the song is `+0x94`). The song's global struct (`song+0x158`,
//! [`song::Globals`]) holds the transport state the sequencer reads. [`Sequencer::tick`] is
//! `sub_1001EF20`, which the mixer block `sub_10013444` runs once per block before any device
//! renders (0x1001346A). Every device call made here therefore lands between two blocks: the
//! device sees it at sample 0 of the block that follows.
//!
//! A track (ctor `sub_1001EDA0`, 0x22C bytes) is a sorted circular list (list ctor `sub_100144B0`
//! at `track+0x108`: the head node is the list itself, with tick `0x7FFFFFFF`), a pointer to the
//! last applied node (`+0x228`), the chase state (`+0x84`, 128 bytes), a per-controller hold mask
//! (`+4`, 128 bytes) and the device id (`+0x104`). [`Track`] keeps the list in a `Vec` in list
//! order; `next` is the index of the node after the last applied one.

#[cfg(test)]
mod tests;

use crate::frame::Devices;
use crate::song::{self, Device, Globals};

/// `dbl_10153808`: the position step per block per BPM*1000, `64*192 / (60*44100*1000)`.
pub const TEMPO_STEP: f64 = f64::from_bits(0x3ED3_7A72_81B2_2126); // 4.6439909297052154e-06
/// `dbl_10153810`: the song ends when the position passes this (999 bars minus a tick).
pub const SONG_END: f64 = f64::from_bits(0x4127_69FE_0000_0000); // 767231.0
/// `sub_10014750` drops an event whose quantised tick is above this (`cmp eax, 0BB4FFh`).
pub const LAST_EVENT_TICK: i32 = 767_231;
/// Ticks per bar: stop relocates to `768 * (tick / 768)` (`sub_1001E100`).
const BAR: i32 = song::TICKS_PER_BAR as i32;
/// The shuffle period: one 8th note (`mov edx, 60h` at 0x1001F3E8).
const SHUFFLE_PERIOD: i32 = 96;
/// The shuffle amount's divisor (`mov ecx, 6` at 0x1001F3D7).
const SHUFFLE_DIV: i32 = 6;

/// `trunc(pos)`: `fistp qword` under a truncating control word, of which the low dword is used
/// (0x1001EF88, `sub_1001F870`).
pub fn trunc(pos: f64) -> i32 {
    pos as i64 as i32
}

/// The shuffled tick of 0x1001F3D0..0x1001F429: `96*(t/96) + max(0, t%96 - shuffle/6)`, with
/// `idiv`'s truncating division. Only the second 16th of each 8th moves, by up to 21 ticks.
pub fn shuffled(tick: i32, shuffle: u8) -> i32 {
    let s = shuffle as i32 / SHUFFLE_DIV;
    let r = (tick % SHUFFLE_PERIOD - s).max(0);
    SHUFFLE_PERIOD * (tick / SHUFFLE_PERIOD) + r
}

/// One node of a track's list: `+0` next, `+4` tick, `+8` controller, `+9` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    pub tick: i32,
    pub ctrl: u8,
    pub value: u8,
}

/// One track: the events of one device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    /// `+0x104`: the device whose set-param (slot 11) the events go to.
    pub device: Device,
    /// The list after its head, in list order.
    events: Vec<Event>,
    /// The index of the node after the one `+0x228` points at (0 when it points at the head).
    next: usize,
    /// `+4`: a controller whose byte is set is skipped by the clock's apply loops (not by the
    /// chase). Zero from the ctor; set and cleared only by the recording path (`sub_1001F520`
    /// clears it), which the load and the export never reach.
    pub hold: [bool; 128],
    /// List `+0xD`: each controller's quantisation in 24-tick units, 1 from the list ctor.
    quant: [u8; 128],
}

impl Track {
    pub fn new(device: Device) -> Self {
        Track {
            device,
            events: Vec::new(),
            next: 0,
            hold: [false; 128],
            quant: [1; 128],
        }
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// The index of the next event the clock applies.
    pub fn next(&self) -> usize {
        self.next
    }

    /// `sub_100146E4`: the tick rounded up to a multiple of `24*q`. q is 1 from the list ctor, so
    /// a loaded tick, already a multiple of 24, is unchanged, except on the 303 tracks, whose
    /// controllers 0 and 1 have q = 32 (a bar) from the 303's slot 7 ([`Track::set_quant`]).
    fn quantise(&self, tick: i32, ctrl: u8) -> i32 {
        let q = self.quant.get(ctrl as usize).copied().unwrap_or(1) as i32;
        let unit = 24 * q;
        // lea ebx, [edx+eax*8-1]; cdq; idiv ecx; imul eax, ecx
        tick.wrapping_add(unit).wrapping_sub(1) / unit * unit
    }

    /// `sub_10014600(list, ctrl, q)`: controller `ctrl`'s quantisation becomes `q` 24-tick units
    /// (the 303's slot 7 sets 32 for controllers 0 and 1).
    pub fn set_quant(&mut self, ctrl: u8, q: u8) {
        self.quant[ctrl as usize & 127] = q;
    }

    /// `sub_10014750`, the insert of song msg 37 (`sub_10021010`): the tick is quantised and an
    /// event above [`LAST_EVENT_TICK`] is dropped. The new node goes before the first node whose
    /// tick is not below it, unless a node of the same tick has the same controller, whose value
    /// is then replaced. So events of one tick end up in reverse arrival order. Returns whether
    /// the event was kept.
    pub fn insert(&mut self, tick: i32, ctrl: u8, value: u8) -> bool {
        let t = self.quantise(tick, ctrl);
        if t > LAST_EVENT_TICK {
            return false;
        }
        let at = self.events.partition_point(|e| t > e.tick);
        if let Some(e) = self.events[at..]
            .iter_mut()
            .take_while(|e| e.tick == t)
            .find(|e| e.ctrl == ctrl)
        {
            e.value = value;
            return true;
        }
        self.events.insert(
            at,
            Event {
                tick: t,
                ctrl,
                value,
            },
        );
        // a node inserted before the applied one moves it; one right after it becomes its next
        if at < self.next {
            self.next += 1;
        }
        true
    }

    /// The chase `sub_100151F0(list, state, p)`: the value of the last event at or before `p` for
    /// each controller (0xFF for none), and the applied pointer moved to that event.
    pub fn chase(&mut self, p: i32) -> [u8; 128] {
        let mut state = [0xFF; 128];
        let mut i = 0;
        while i < self.events.len() && p >= self.events[i].tick {
            let e = self.events[i];
            state[e.ctrl as usize & 127] = e.value;
            i += 1;
        }
        self.next = i;
        state
    }

    /// The clock's apply loop (0x1001F2C6, 0x1001F032, 0x1001FADB): every event after the applied
    /// one with `tick <= t`, in list order, skipping held controllers.
    pub fn advance(&mut self, t: i32, mut apply: impl FnMut(Event)) {
        while self.next < self.events.len() && t >= self.events[self.next].tick {
            let e = self.events[self.next];
            if !self.hold[e.ctrl as usize & 127] {
                apply(e);
            }
            self.next += 1;
        }
    }

    /// `sub_100149F0`, run on every track by stop: an event whose value equals the previous
    /// value of its controller (0xFF before the first) is removed, unless it is at tick 0.
    pub fn prune(&mut self) {
        let mut last = [0xFFu8; 128];
        let mut i = 0;
        while i < self.events.len() {
            let e = self.events[i];
            let c = e.ctrl as usize & 127;
            if e.value != last[c] {
                last[c] = e.value;
                i += 1;
            } else if e.tick != 0 {
                self.events.remove(i);
                // the DLL frees the node; the applied pointer then moves back to its predecessor
                if i < self.next {
                    self.next -= 1;
                }
            } else {
                i += 1;
            }
        }
    }
}

/// The song's sequencer and transport state.
#[derive(Debug, Clone, PartialEq)]
pub struct Sequencer {
    /// `seq+0`: the position in ticks.
    pub pos: f64,
    /// `seq+8`: the tick of the last block that did work.
    pub last_tick: i32,
    /// `seq+0x10`: the song-mode tick saved when pattern mode was entered.
    pub saved_tick: i32,
    /// The tracks of devices 0..8 (`sub_1001EEF0(i, i, n)` in the song ctor).
    pub tracks: [Track; 9],
    /// `song+0x158`: song mode, play, record, loop, tempo, loop points, shuffle. The sound-model
    /// byte (`+0x16D`) is the frame's `vintage`.
    pub globals: Globals,
}

impl Default for Sequencer {
    /// The ctors' values: `sub_1001EC50` (position, ticks 0) and the global struct's
    /// `sub_10018AC0` (tempo 120000, loop end 768, shuffle 64, byte 22 = 1, the rest 0).
    fn default() -> Self {
        Sequencer {
            pos: 0.0,
            last_tick: 0,
            saved_tick: 0,
            tracks: Device::ALL.map(Track::new),
            globals: Globals {
                song_mode: 0,
                play: 0,
                byte2: 0,
                loop_on: 0,
                tempo: 120_000,
                locate: 0,
                loop_start: 0,
                loop_end: 768,
                shuffle: 64,
                vintage: 0,
                byte22: 1,
            },
        }
    }
}

impl Sequencer {
    fn song_mode(&self) -> bool {
        self.globals.song_mode != 0
    }

    fn playing(&self) -> bool {
        self.globals.play != 0
    }

    /// The tick of the position (`sub_1001F870`).
    pub fn tick_pos(&self) -> i32 {
        trunc(self.pos)
    }

    /// Chases every track to `p` and applies each controller that has a value, ascending, to
    /// its device (the loop at 0x1001FA2C; `sub_10012B40` finds the device by id).
    fn chase_all(&mut self, p: i32, devices: &mut Devices) {
        for tr in &mut self.tracks {
            let state = tr.chase(p);
            let d = devices.device_mut(tr.device);
            for (c, &v) in state.iter().enumerate() {
                if v != 0xFF {
                    d.set_param(c as u8, v);
                }
            }
        }
    }

    /// The apply loop over every track, in track order.
    fn advance_all(&mut self, t: i32, devices: &mut Devices) {
        for tr in &mut self.tracks {
            let d = devices.device_mut(tr.device);
            tr.advance(t, |e| d.set_param(e.ctrl, e.value));
        }
    }

    /// `sub_1001EF20`, the clock, once per block before the devices render:
    /// ```text
    /// if !play: return
    /// inc = tempo * TEMPO_STEP; pos = pos + inc; t = trunc(pos)
    /// if t > lastTick:
    ///     if song mode:
    ///         if loop on and loopEnd <= pos and !(loopEnd <= pos - inc):
    ///             apply events up to loopEnd - 1
    ///             pos = pos + (loopStart - loopEnd); t = trunc(pos)
    ///             chase to loopStart, relocate every device to loopStart
    ///         apply events up to t
    ///         if pos > 767231: song end (stop, locate 0), return
    ///     every device: step(t, shuffled(t), tempo)
    /// lastTick = t
    /// ```
    /// The tempo of the position step is the current song's (`sub_1001BCC0` without MIDI sync),
    /// which is this song's whenever it renders. Returns whether it reached the step loop,
    /// whose tempo argument it stores in a local at 0x1001F3B3 (`esp+0x34`, the address of
    /// [`crate::frame::Frame::stack_word`]).
    pub fn tick(&mut self, devices: &mut Devices) -> bool {
        if !self.playing() {
            return false;
        }
        let tempo = self.globals.tempo as i32;
        // 0x1001EF51: fild tempo; fmul; fld pos; fadd st, st(1)
        let inc = tempo as f64 * TEMPO_STEP;
        self.pos += inc;
        let mut t = trunc(self.pos);
        if t > self.last_tick {
            if self.song_mode() {
                let le = self.globals.loop_end as i32;
                // 0x1001EFD6: loopEnd <= pos; 0x1001EFF3: loopEnd > pos - inc
                if self.globals.loop_on != 0 && le as f64 <= self.pos && le as f64 > self.pos - inc
                {
                    self.advance_all(le.wrapping_sub(1), devices);
                    // 0x1001F0C7: global byte 2 (record) would merge the recorded events here
                    // (`sub_1001F520`); nothing the load or the export sends sets it
                    let ls = self.globals.loop_start as i32;
                    // 0x1001F135: fld pos; fild (start - end); faddp
                    self.pos += ls.wrapping_sub(le) as f64;
                    t = trunc(self.pos);
                    // 0x1001F182 tests song mode again, which holds here
                    self.chase_all(ls, devices);
                    devices.for_each(|d| d.relocate(ls, t));
                }
                self.advance_all(t, devices);
                // 0x1001F35F: pos > 767231
                if self.pos > SONG_END {
                    self.song_end(devices);
                    return false;
                }
            }
            let sp = shuffled(t, self.globals.shuffle);
            let tempo = self.globals.tempo as i32;
            devices.for_each(|d| d.step(t, sp, tempo));
            self.last_tick = t;
            return true;
        }
        self.last_tick = t;
        false
    }

    /// 0x1001F484: record off (`sub_10021194(0)`), the host is told (msg 35 `{2,0}` and
    /// `{1,0}`), stop (`sub_100218A0`, without the bar locate) and locate to 0.
    fn song_end(&mut self, devices: &mut Devices) {
        self.globals.byte2 = 0;
        self.halt(devices);
        self.locate(0, devices);
    }

    /// `sub_1001F984`, locate (song msg 35 `{8,p}`, msg 32): a negative tick becomes 0; in song
    /// mode every track is chased to it (with record on, stopped and a later tick, the events up
    /// to it are applied instead); then the position and the last tick become it and every
    /// device is relocated (slot 2).
    pub fn locate(&mut self, p: i32, devices: &mut Devices) {
        let p = p.max(0);
        if self.globals.byte2 != 0 && !self.playing() && p > self.tick_pos() {
            self.advance_all(p, devices);
        } else if self.song_mode() {
            self.chase_all(p, devices);
        }
        self.pos = p as f64;
        devices.for_each(|d| d.relocate(p, p));
        self.last_tick = p;
    }

    /// `sub_100218A0`: when playing, clear the play flag and stop every device (slot 4); prune
    /// every track; in song mode chase every track to the current tick.
    pub fn halt(&mut self, devices: &mut Devices) {
        if self.playing() {
            self.globals.play = 0;
            devices.for_each(|d| d.stop());
        }
        for tr in &mut self.tracks {
            tr.prune();
        }
        if self.song_mode() {
            let t = self.tick_pos();
            self.chase_all(t, devices);
        }
    }

    /// Song msg 35 `{1,0}`, `sub_1001E100` (no ReWire, no MIDI sync): `sub_100218A0`, then
    /// locate to the start of the current bar.
    pub fn stop(&mut self, devices: &mut Devices) {
        self.halt(devices);
        let t = self.tick_pos();
        self.locate(BAR * (t / BAR), devices);
    }

    /// Song msg 35 `{1,1}`, `sub_1001BE74` → 0x1001F7E4: when stopped, set the play flag and
    /// start every device (slot 3). The position is not touched.
    pub fn play(&mut self, devices: &mut Devices) {
        if !self.playing() {
            self.globals.play = 1;
            let now = self.tick_pos();
            devices.for_each(|d| d.start(now));
        }
    }

    /// Song msg 35 `{0,m}`, `sub_1001FBE0`: on a change, store the byte and tell every device
    /// (slot 9, `sub_10013FB0`); then song mode locates to the saved tick, pattern mode relocates
    /// every device to 0 and saves the current tick.
    pub fn set_song_mode(&mut self, mode: u8, devices: &mut Devices) {
        if self.globals.song_mode != mode {
            self.globals.song_mode = mode;
            devices.for_each(|d| d.set_song_mode(mode != 0));
        }
        if mode != 0 {
            self.locate(self.saved_tick, devices);
        } else {
            let now = self.tick_pos();
            devices.for_each(|d| d.relocate(0, now));
            self.saved_tick = self.tick_pos();
        }
    }

    /// Song msg 35 `{2,v}`, `sub_10021194`: store the record byte. Turning record off while on
    /// would merge the recording and chase (not modelled: nothing sets it).
    pub fn set_record(&mut self, v: u8) {
        self.globals.byte2 = v;
    }

    /// Song msg 35 `{3,v}` (`sub_10021110`).
    pub fn set_loop_on(&mut self, v: u8) {
        self.globals.loop_on = v;
    }

    /// Song msg 35 `{4,v}` (`sub_1001FCD0`), BPM*1000.
    pub fn set_tempo(&mut self, v: u32) {
        self.globals.tempo = v;
    }

    /// Song msg 35 `{12,v}` (`sub_10021130`).
    pub fn set_loop_start(&mut self, v: u32) {
        self.globals.loop_start = v;
    }

    /// Song msg 35 `{16,v}` (`sub_10021150`).
    pub fn set_loop_end(&mut self, v: u32) {
        self.globals.loop_end = v;
    }

    /// Song msg 35 `{20,v}` (`sub_10021170`).
    pub fn set_shuffle(&mut self, v: u8) {
        self.globals.shuffle = v;
    }

    /// Song msg 37 (`sub_10021010`): each event of a batch is inserted into its track.
    pub fn add_events(&mut self, track: usize, events: &[song::TrackEvent]) {
        let tr = &mut self.tracks[track];
        for e in events {
            tr.insert(e.tick as i32, e.ctrl, e.value);
        }
    }
}
