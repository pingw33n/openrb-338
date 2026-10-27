//! The interaction math of the original's widgets (docs/ui_spec.md section 2), as pure
//! functions on panel pixels so that it is unit-tested without a window.

/// Highest knob position (the knob keeps p in 0..=1000).
pub const KNOB_MAX: i32 = 1000;
/// Highest fader position (0..=999).
pub const FADER_MAX: i32 = 999;
/// Fader track height, handle height and the handle's travel, in panel pixels.
pub const FADER_TRACK: i32 = 54;
pub const FADER_HANDLE: i32 = 11;
pub const FADER_TRAVEL: f32 = 43.0;
/// The fader's scale `k = 10000 / 43` (integer, `CBitmapFaderView`).
const FADER_K: i32 = 232;

/// Pixels of vertical drag for the knob's full range, by control id
/// (the ctor's fourth argument): TB-303 62, TR-808 36 (selector 40), TR-909 41, others 64.
pub fn knob_span(id: &str) -> f32 {
    if id == "tr808.selector" {
        return 40.0;
    }
    match id.split('.').next().unwrap_or("") {
        "tb303_1" | "tb303_2" => 62.0,
        "tr808" => 36.0,
        "tr909" => 41.0,
        _ => 64.0,
    }
}

/// The knob position after a drag: grabbed at `p0` with the pointer at `y0`, now at `y`
/// (panel pixels). Up increases; the span is 5 times larger with Shift held at mouse down.
pub fn knob_drag(p0: i32, y0: f32, y: f32, span: f32, fine: bool) -> i32 {
    let span = if fine { span * 5.0 } else { span };
    let d = (KNOB_MAX as f32 * (y0 - y) / span) as i32;
    (p0 + d).clamp(0, KNOB_MAX)
}

/// The value 0..=127 sent for the position (`sub_467F1A`).
pub fn knob_send(p: i32) -> i32 {
    (127 * p + 500) / 1000
}

/// The position for a value from the engine (`sub_4A637D`).
pub fn knob_from_engine(v: i32) -> i32 {
    (1000 * v + 63) / 127
}

/// The pointer angle of a position in degrees: 0 = up, clockwise positive, -135 to +135.
pub fn knob_angle(p: i32) -> f32 {
    -135.0 + 270.0 * p as f32 / KNOB_MAX as f32
}

/// The value 0..=127 sent for a fader position (`sub_469614`).
pub fn fader_send(p: i32) -> i32 {
    (127 * p + 500) / 1000
}

/// The handle's top edge, from the track's top (panel pixels).
pub fn fader_handle_top(p: i32) -> f32 {
    (1.0 - p as f32 / FADER_MAX as f32) * FADER_TRAVEL
}

/// Whether a point `y` (from the track's top) is on the handle at position `p`.
pub fn fader_on_handle(p: i32, y: f32) -> bool {
    let top = fader_handle_top(p);
    y >= top && y < top + FADER_HANDLE as f32
}

/// Mouse down on the track outside the handle: the handle's center jumps to the pointer
/// (`p = (bottom - y - 11/2 + 2) * k / 10`, integer, clamped).
pub fn fader_jump(y: f32) -> i32 {
    let y = y.floor() as i32;
    ((FADER_TRACK - y - FADER_HANDLE / 2 + 2) * FADER_K / 10).clamp(0, FADER_MAX)
}

/// The fader position after a drag of the handle (up increases, no fine mode).
pub fn fader_drag(p0: i32, y0: f32, y: f32) -> i32 {
    (p0 + (FADER_K as f32 * (y0 - y) / 10.0) as i32).clamp(0, FADER_MAX)
}

/// The extra steps of a held repeat button after `t` ms (`sub_4A6C30`): one per 100 ms from
/// 400 ms, and from 2400 ms one more per 20 ms. The mouse down itself gives one step.
pub fn repeat_extra_steps(t_ms: u32) -> u32 {
    let a = if t_ms >= 400 {
        (t_ms - 400).min(2399) / 100
    } else {
        0
    };
    let b = if t_ms >= 2400 { (t_ms - 2400) / 20 } else { 0 };
    a + b
}

/// A two-position switch's value for a pointer at `pos` along its axis: the first half gives
/// 0, the second 1 (`CBitmapSwitch`).
pub fn switch_value(pos: f32, size: f32) -> u8 {
    u8::from(pos >= size / 2.0)
}

/// The pattern grid's button 1..=8 for a point in its 68 x 40 area (`sub_41A5B0`).
pub fn pattern_grid_button(dx: f32, dy: f32) -> u8 {
    let col = ((4.0 * dx / 68.0) as i32).clamp(0, 3);
    let row = ((2.0 * dy / 40.0) as i32).clamp(0, 1);
    (4 * row + col + 1) as u8
}

/// The modifier keys that matter for the TR-909's step buttons.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Mods {
    pub ctrl: bool,
    pub alt: bool,
}

/// The new value of a TR-909 step on mouse down, from its value `c` (`CTR909PatternView`):
/// accent: off <-> 2; Ctrl+Alt or the flam button: off <-> 3; Ctrl: off <-> 1; Alt: off <-> 2;
/// otherwise off -> 1 -> 2 -> off.
pub fn tr909_step_value(c: u8, accent_row: bool, flam_on: bool, m: Mods) -> u8 {
    let toggle = |on_value: u8| if c == on_value { 0 } else { on_value };
    if accent_row {
        return if c != 0 { 0 } else { 2 };
    }
    if (m.ctrl && m.alt) || flam_on {
        toggle(3)
    } else if m.ctrl {
        toggle(1)
    } else if m.alt {
        toggle(2)
    } else {
        (c + 1) % 3
    }
}

/// The TR-808 instrument selected by the instrument selector knob's value 0..=127.
pub fn selector_instrument(v: i32) -> usize {
    ((11 * v + 63) / 127) as usize
}

/// The selector knob's value 0..=127 that shows instrument `i`.
pub fn selector_value(i: usize) -> i32 {
    (127 * i as i32 + 5) / 11
}

/// The TR-808 instrument under a point of its section, from the legend row
/// (x 10..274, y 71..85: 12 cells of 22 px) or the upper labels (x 78..184, y 43..55: the
/// slots 3..7).
pub fn tr808_hit(x: f32, y: f32) -> Option<usize> {
    if (71.0..85.0).contains(&y) && (10.0..274.0).contains(&x) {
        return Some(((x - 10.0) / 22.0) as usize);
    }
    if (43.0..55.0).contains(&y) && (78.0..184.0).contains(&x) {
        return Some(((x - 78.0) / 22.0) as usize + 3);
    }
    None
}

/// The TR-909 instrument under a point of its section: the legend row (y 4..18), the name
/// row (y 70..85) and the AC area (4..26, 44..62).
pub fn tr909_hit(x: f32, y: f32) -> Option<usize> {
    if (4.0..26.0).contains(&x) && (44.0..62.0).contains(&y) {
        return Some(0);
    }
    if !((4.0..18.0).contains(&y) || (70.0..85.0).contains(&y)) {
        return None;
    }
    let dx = x as i32;
    let i = if dx < 26 {
        0
    } else if dx < 236 {
        (dx - 26) / 42 + 1
    } else {
        (dx - 236) / 21 + 6
    };
    (0..=11).contains(&i).then_some(i as usize)
}

/// The TR-808 step under `x` (section pixels): 16 cells of 16 px from x 16.
pub fn tr808_step(x: f32) -> Option<usize> {
    let i = ((x - 16.0) / 16.0).floor();
    (0.0..16.0).contains(&i).then_some(i as usize)
}

/// The TR-909 step under `x` (section pixels): 16 cells of 21 px from x 29.
pub fn tr909_step(x: f32) -> Option<usize> {
    let i = ((x - 29.0) / 21.0).floor();
    (0.0..16.0).contains(&i).then_some(i as usize)
}

/// The frame names of a seven-segment display of `n` digits showing `value`, right-aligned
/// with blank leading zeros; `None` is a disabled display (all blank).
pub fn digit_frames(value: Option<i32>, n: usize) -> Vec<&'static str> {
    const NAMES: [&str; 10] = [
        "seg_0", "seg_1", "seg_2", "seg_3", "seg_4", "seg_5", "seg_6", "seg_7", "seg_8", "seg_9",
    ];
    let mut frames = vec!["seg_blank"; n];
    let Some(v) = value else {
        return frames;
    };
    let mut v = v.clamp(0, 10_i32.pow(n as u32) - 1);
    for i in (0..n).rev() {
        frames[i] = NAMES[(v % 10) as usize];
        v /= 10;
        if v == 0 {
            break;
        }
    }
    frames
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knob_cutoff_measured_in_the_original() {
        // "CUTOFF dragged to 0, then 31 px up, saved as 64".
        let p = knob_drag(0, 100.0, 69.0, knob_span("tb303_1.cutoff"), false);
        assert_eq!(p, 500);
        assert_eq!(knob_send(p), 64);
        // "RESO with Shift, 31 px up from 0, saved as 13".
        let p = knob_drag(0, 100.0, 69.0, knob_span("tb303_1.reso"), true);
        assert_eq!(p, 100);
        assert_eq!(knob_send(p), 13);
    }

    #[test]
    fn knob_clamps_ignores_nothing_and_has_per_machine_spans() {
        assert_eq!(knob_drag(900, 0.0, -1000.0, 64.0, false), KNOB_MAX);
        assert_eq!(knob_drag(100, 0.0, 1000.0, 64.0, false), 0);
        // Down decreases.
        assert!(knob_drag(500, 0.0, 10.0, 64.0, false) < 500);
        assert_eq!(knob_span("tb303_2.tune"), 62.0);
        assert_eq!(knob_span("tr808.bd_level"), 36.0);
        assert_eq!(knob_span("tr808.selector"), 40.0);
        assert_eq!(knob_span("tr909.flam"), 41.0);
        assert_eq!(knob_span("mixer3.pan"), 64.0);
        assert_eq!(knob_span("transport.shuffle"), 64.0);
    }

    #[test]
    fn knob_value_round_trip() {
        assert_eq!(knob_send(0), 0);
        assert_eq!(knob_send(1000), 127);
        for v in 0..=127 {
            assert_eq!(knob_send(knob_from_engine(v)), v);
        }
        assert_eq!(knob_angle(0), -135.0);
        assert_eq!(knob_angle(500), 0.0);
        assert_eq!(knob_angle(1000), 135.0);
    }

    #[test]
    fn fader_jump_drag_and_handle() {
        assert_eq!(fader_handle_top(FADER_MAX), 0.0);
        assert_eq!(fader_handle_top(0), 43.0);
        assert!(fader_on_handle(0, 45.0));
        assert!(!fader_on_handle(0, 30.0));
        assert_eq!(fader_jump(0.0), FADER_MAX);
        assert_eq!(fader_jump(54.0), 0);
        assert_eq!(fader_jump(5.0), FADER_MAX);
        // Mid-track.
        assert_eq!(fader_jump(27.0), (54 - 27 - 5 + 2) * 232 / 10);
        // Dragging the full travel up from the bottom reaches the top (within 3 steps).
        assert!(fader_drag(0, 50.0, 7.0) >= 990);
        assert_eq!(fader_drag(0, 50.0, 60.0), 0);
        assert_eq!(fader_send(FADER_MAX), 127);
    }

    #[test]
    fn repeat_timing() {
        assert_eq!(repeat_extra_steps(0), 0);
        assert_eq!(repeat_extra_steps(399), 0);
        assert_eq!(repeat_extra_steps(400), 0);
        assert_eq!(repeat_extra_steps(500), 1);
        assert_eq!(repeat_extra_steps(1400), 10);
        assert_eq!(repeat_extra_steps(2399), 19);
        assert_eq!(repeat_extra_steps(2400), 20);
        // From 2400 ms one more per 20 ms: 22 (capped 100 ms steps at 2200 ms) + 10.
        assert_eq!(repeat_extra_steps(2400 + 200), 22 + 10);
    }

    #[test]
    fn switches_and_grid() {
        assert_eq!(switch_value(2.0, 16.0), 0);
        assert_eq!(switch_value(8.0, 16.0), 1);
        assert_eq!(pattern_grid_button(1.0, 1.0), 1);
        assert_eq!(pattern_grid_button(67.0, 1.0), 4);
        assert_eq!(pattern_grid_button(1.0, 39.0), 5);
        assert_eq!(pattern_grid_button(67.0, 39.0), 8);
        assert_eq!(pattern_grid_button(-5.0, 90.0), 5);
    }

    #[test]
    fn tr909_step_cycle_and_modifiers() {
        let none = Mods::default();
        assert_eq!(tr909_step_value(0, false, false, none), 1);
        assert_eq!(tr909_step_value(1, false, false, none), 2);
        assert_eq!(tr909_step_value(2, false, false, none), 0);
        let ca = Mods {
            ctrl: true,
            alt: true,
        };
        assert_eq!(tr909_step_value(0, false, false, ca), 3);
        assert_eq!(tr909_step_value(3, false, false, ca), 0);
        let c = Mods {
            ctrl: true,
            alt: false,
        };
        assert_eq!(tr909_step_value(0, false, false, c), 1);
        assert_eq!(tr909_step_value(1, false, false, c), 0);
        let a = Mods {
            ctrl: false,
            alt: true,
        };
        assert_eq!(tr909_step_value(2, false, false, a), 0);
        assert_eq!(tr909_step_value(0, false, false, a), 2);
        assert_eq!(tr909_step_value(0, false, true, none), 3);
        // The accent row is on/off with the stored "on" = 2.
        assert_eq!(tr909_step_value(0, true, false, none), 2);
        assert_eq!(tr909_step_value(1, true, false, none), 0);
    }

    #[test]
    fn instrument_selection() {
        for i in 0..12 {
            assert_eq!(selector_instrument(selector_value(i)), i);
        }
        assert_eq!(tr808_hit(10.0, 75.0), Some(0));
        assert_eq!(tr808_hit(273.0, 75.0), Some(11));
        assert_eq!(tr808_hit(274.0, 75.0), None);
        assert_eq!(tr808_hit(78.0, 50.0), Some(3));
        assert_eq!(tr808_hit(183.0, 50.0), Some(7));
        assert_eq!(tr808_hit(50.0, 100.0), None);
        assert_eq!(tr909_hit(10.0, 10.0), Some(0));
        assert_eq!(tr909_hit(26.0, 10.0), Some(1));
        assert_eq!(tr909_hit(235.0, 10.0), Some(5));
        assert_eq!(tr909_hit(236.0, 75.0), Some(6));
        assert_eq!(tr909_hit(361.0, 75.0), Some(11));
        assert_eq!(tr909_hit(362.0, 10.0), None);
        assert_eq!(tr909_hit(10.0, 50.0), Some(0));
        assert_eq!(tr909_hit(100.0, 50.0), None);
        assert_eq!(tr808_step(15.0), None);
        assert_eq!(tr808_step(16.0), Some(0));
        assert_eq!(tr808_step(271.0), Some(15));
        assert_eq!(tr808_step(272.0), None);
        assert_eq!(tr909_step(29.0), Some(0));
        assert_eq!(tr909_step(364.0), Some(15));
        assert_eq!(tr909_step(365.0), None);
    }

    #[test]
    fn digits_right_aligned_with_blank_leading_zeros() {
        assert_eq!(digit_frames(Some(140), 3), ["seg_1", "seg_4", "seg_0"]);
        assert_eq!(digit_frames(Some(3), 2), ["seg_blank", "seg_3"]);
        assert_eq!(digit_frames(Some(0), 2), ["seg_blank", "seg_0"]);
        assert_eq!(digit_frames(None, 2), ["seg_blank", "seg_blank"]);
        assert_eq!(digit_frames(Some(1000), 3), ["seg_9", "seg_9", "seg_9"]);
    }
}
