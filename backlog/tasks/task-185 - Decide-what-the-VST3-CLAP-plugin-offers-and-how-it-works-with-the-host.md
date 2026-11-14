---
id: TASK-185
title: 'Decide: what the VST3/CLAP plugin offers and how it works with the host'
status: To Do
assignee:
  - '@pingw33n'
created_date: '2026-11-14T07:36:00Z'
labels:
  - decision
  - ui
  - player
milestone: m-7
dependencies: []
references:
  - docs/ui_spec.md
  - docs/song_load.md
  - docs/initial_investigation.md
  - crates/orb-player/src/lib.rs
  - crates/orb-ui/src/menu.rs
  - https://github.com/robbert-vdh/nih-plug
priority: medium
type: spike
ordinal: 201000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
orb is to become a VST3/CLAP instrument plugin (nih-plug, both formats from one codebase) in m-7, next to MIDI input (task-182 remote control, task-183 live play, task-184 clock sync for the standalone app only). This task is the user's survey of similar plugins and the product decisions the plugin's implementation tasks need. Below: what the discussion of 2026-11-13/14 found, then each decision with options and a recommendation.

## Facts from the discussion
- Engine: runs only at 44.1 kHz, in 64-frame blocks; the player applies commands per 1024-frame call (the original's MME buffer). orb-player's `Player`/`Audio` split (lock-free, no allocation in the callback, a garbage queue) fits a plugin; commands should apply per 64-frame block at event offsets.
- A song (`.rbs`) = patterns (device data, edited live, read live in both modes) + tracks of controller events (knobs, pattern select ctrl 1, on/off ctrl 0). Song-mode record records controller moves only: never pattern contents, mixer mutes, the master, tempo, shuffle or loop points (docs/song_load.md 4.10, task-057). So patterns are like a DAW's shared clips, the song like its arrangement.
- The original has a slave mode: with MIDI sync active, Stop does not relocate (`transport+0x160`); `transport+0x15C == 46` (ReWire, [G]) leaves loop and tempo to the host.
- Neither VST3 nor CLAP lets a plugin edit host clips or automation lanes. Precedent for exchanging patterns: MIDI drag-out (D16, AudioRealism, XO, Toontrack), dropping MIDI files on the plugin, plugin MIDI output recorded by the host (Bitwig, Reaper, FL, Cubase; Ableton limited), host-specific scripting (Max for Live, Bitwig API, ReaScript).
- DAWs already model patterns: FL Studio (patterns + playlist, slide notes), Studio One (Pattern events with variations), Logic (Step Sequencer rows incl. tie and octave, aliases), Cubase shared copies, Reaper pooled items; Ableton/Bitwig clip launchers with bar launch quantization = ReBirth's bar-line latch.
- 303 note conventions of every clone: pitch = note, accent = velocity at or above a threshold, slide = overlapping notes (causal, no lookahead), gate = note length.
- The DLL has a 303 MIDI input (`sub_100261F0`); whether the 808/909 played from notes is task-183's question.
- Mods: recrypted `.rbm`, 2.6 MB (Standard, embedded in orb) to 9.4 MB, about 3.4 MB on average over the user's 320 third-party mods.
- UI: orb-ui is eframe 0.37 + wgpu; nih_plug_egui runs an older egui on baseview/OpenGL, so the editor needs an updated egui-baseview or our own baseview+wgpu glue. No native menu bar in a plugin window, rfd dialogs risky, hosts eat keyboard shortcuts. The panel already fits any size with the aspect kept.

## Survey (to do)
Starting list, from memory, to check: Reason Rack Plugin (whole rack in one instance, same company as ReBirth); D16 Phoscyon 2 / Nepheton / Drumazon / Nithonat (one device per instance, patterns synced to host, pattern switch by low MIDI notes, MIDI drag-out, multi-out); AudioRealism ABL3 / ADM; Roland Cloud TB-303 / TR-808 / TR-909; Native Instruments Maschine (patterns and its own arrangement inside a DAW); XLN XO; Open303 / JC303 (open source, note conventions). For each: instance model, sequencer vs host notes, pattern switching, transport/song handling, MIDI export/import, multi-out, how samples/kits are stored.

## Decisions
1. **What the panel shows and does in plugin mode.** Transport strip (docs/ui_spec.md 3). Recommended:
   - keep Shuffle (plugin state, automatable) and the Pattern/Song switch;
   - tempo display shows the host tempo, arrows disabled;
   - Play/Stop/Rewind/FF, bar up/down and the loop controls disabled (dimmed): the host owns the transport;
   - bar display shows the host position;
   - Record disabled (see 2);
   - MIDI LED lights on host MIDI; Sync LED lit while following the host;
   - logo click (startup sound) kept, the startup sound off by default.

   Menus (in the panel, no native bar):
   - File: New, Open (load an .rbs into the instance), Save As (write an .rbs), Info; no Save (state lives in the project); no Export loop as audio file (the host bounces).
   - Mods: as in the app, Demo Song included.
   - Options: Vintage, Play Mod Startup Sound, key mode (best effort), plus zoom (75–200 %) and the MIDI settings (channel map, accent threshold, per-device mode).
   - Help: as is.
2. **Patterns vs host notes.** Options: (a) internal sequencer only, host gives transport; (b) host notes only; (c) per device a mode switch, Pattern (internal sequencer, host transport) or Notes (host notes, that device's pattern silent), mixable in one instance. Recommended: (c), Pattern by default.
   - Pattern selection from the host: an automatable parameter per device, plus MIDI notes or program changes per task-182's map, latched at the bar as in the original.
   - Editor: always edits the device's stored patterns. It follows the selected pattern when host automation changes it, as song mode does in the original. Edits are heard on the next pass. In Notes mode it still edits (not heard) and says so.
   - Notes on a device in Pattern mode: whatever task-183 finds the original did, else ignored.
   - Song mode follows the host position (ppq × 192 = ticks, locate and chase on jumps); a host-automated controller holds that controller against the song's track; no stop at the song end; ReBirth's own loop ignored.
   - Recording into song tracks: not in the plugin; the host records automation.
3. **Live note input.** Options: MIDI channels per device on one note input; CLAP note ports / VST3 event buses per device (poor host support for several); one device per instance. Recommended: one note input, the original's channel map, CLAP note events treated like MIDI, no MPE or note expressions; accent threshold configurable, slide by overlap. One-device-per-instance only if users ask.
4. **Export and import with the host.** Recommended:
   - drag a pattern out as a MIDI clip (accent = high velocity, slide = overlap), and "export all patterns" (one clip per pattern);
   - drag the song out as a MIDI arrangement (patterns placed as clips, knobs as CCs);
   - "Save MIDI file…" as the fallback where drag-out fails;
   - import: drop a MIDI clip on a device, quantized to 16ths, into the selected pattern; drop an .rbs to load it.
   - Not now: playing the song into the host's automation recorder.
5. **No direct MIDI devices.** Recommended: the plugin takes MIDI only from the host. Private ports mean the host does not record or replay the input, no sample-accurate timing, exclusive ports on classic Windows MIDI, and every instance receiving everything. Port access and clock sync stay in the standalone app. A possible later exception: LED feedback to a known controller.
6. **Mods in the plugin state.** Options: (a) a reference only (name + SHA-256), resolved from a mod folder setting; (b) the recrypted mod embedded in the state (3–10 MB per instance); (c) both. Recommended: (c). Always store the reference. Embed by default for a third-party mod, with a per-instance toggle, so projects open on another machine. Never embed the built-in Standard kit. Use the embedded copy when the folder lacks the mod. Note that embedding redistributes the mod with the project.

## Also open from the discussion (decide here or as follow-ups)
- Formats: VST3 + CLAP via nih-plug; AU later via clap-wrapper or never. nih-plug is git-only and slow-moving.
- Sample rate: resample to the host rate and report the latency (not bit-exact), or 44.1 kHz only.
- Parameters: stepped integers mirroring the u8 controllers, stable ids.
- Outputs: stereo first; per-device multi-out later (needs engine taps).
- Editor: zoom steps saved in the state, plus aspect-locked resize where the host allows; minimum 100 %; ScrollArea fallback below that.
- Packaging: .vst3/.clap bundles in scripts/package.sh; macOS signing and notarization; validation with clap-validator, the VST3 validator and pluginval.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The survey of similar plugins is in the task notes, each with its instance model, sequencer/notes handling, pattern switching, transport and song handling, MIDI export/import, outputs and sample storage
- [ ] #2 Each of the six decisions, and each item of 'Also open', has an outcome (or an explicit deferral) with the date in the final summary
- [ ] #3 The plugin's implementation tasks are created in m-7 from the outcomes, each depending on this task where it needs a decision
<!-- AC:END -->
