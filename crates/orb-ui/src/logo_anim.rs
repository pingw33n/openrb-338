//! The startup animation of the wordmark (task-114), approved by the user on 2026-10-29:
//!
//! - RB-338 is in place from the start;
//! - OPEN slides left into place over [`SLIDE_S`] with an ease-out, masked to the left of the R, so
//!   it comes out from behind RB-338 at full opacity;
//! - the four underline segments fade in left to right, each over [`SEG_FADE_S`], from
//!   [`SEG_START_S`] on, [`SEG_STEP_S`] apart;
//! - the subtitle fades in over [`SUB_START_S`]..[`SUB_END_S`];
//! - in the About window the icon also fades in over [`ICON_FADE_S`].
//!
//! It plays once per app start on the panel and at every open of the About window. The final
//! frame is the whole `logo` part itself, so it equals the static wordmark exactly. The pieces
//! are the `LOGO_PIECES` of orb-ui-art, each on the whole logo canvas.
//!
//! Developer option: `ORB_LOGO_T=SECONDS` freezes the animation at that time (for screenshots
//! of a moment); it applies to the panel and to the About window alike.

use eframe::egui::{self, Color32, Rect, Ui};

use crate::app;

/// OPEN's slide: its duration.
pub const SLIDE_S: f32 = 1.0;
/// The first underline segment's start, the step to the next, and each one's fade.
pub const SEG_START_S: f32 = 0.55;
pub const SEG_STEP_S: f32 = 0.20;
pub const SEG_FADE_S: f32 = 0.25;
/// The subtitle's fade.
pub const SUB_START_S: f32 = 1.35;
pub const SUB_END_S: f32 = 1.85;
/// The About icon's fade, from 0.
pub const ICON_FADE_S: f32 = 0.6;
/// The end of the whole animation.
pub const END_S: f32 = SUB_END_S;

/// The whole logo part and its size (a test checks them against orb-ui-art's).
pub const LOGO_PART: &str = "logo";
pub const LOGO_W: f32 = 134.0;
pub const LOGO_H: f32 = 48.0;
/// How far OPEN is shifted right at the start: its width and the gap to RB-338 (the x of the R
/// in the part, 8, less the wordmark's left edge; a test checks it against the art's font).
pub const OPEN_TRAVEL: f32 = 49.466_667;
/// The x where RB-338 starts in the logo part: OPEN is clipped to the left of it.
pub const RB_X: f32 = 8.0 + OPEN_TRAVEL;

/// The animation's state at one moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// OPEN's offset to the right of its final place, in logo units.
    pub open_dx: f32,
    /// The underline segments' opacities, left to right.
    pub seg: [f32; 4],
    pub sub: f32,
    pub icon: f32,
}

fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// 0 before `t0`, 1 after `t0 + d`, linear between.
fn ramp(t: f32, t0: f32, d: f32) -> f32 {
    ((t - t0) / d).clamp(0.0, 1.0)
}

impl Frame {
    /// The state `t` seconds after the start.
    pub fn at(t: f32) -> Self {
        let mut seg = [0.0; 4];
        for (i, a) in seg.iter_mut().enumerate() {
            *a = ramp(t, SEG_START_S + SEG_STEP_S * i as f32, SEG_FADE_S);
        }
        Self {
            open_dx: OPEN_TRAVEL * (1.0 - ease_out(ramp(t, 0.0, SLIDE_S))),
            seg,
            sub: ramp(t, SUB_START_S, SUB_END_S - SUB_START_S),
            icon: ramp(t, 0.0, ICON_FADE_S),
        }
    }
}

/// The frozen time of `ORB_LOGO_T`, if set.
pub fn frozen_time() -> Option<f32> {
    static T: std::sync::OnceLock<Option<f32>> = std::sync::OnceLock::new();
    *T.get_or_init(|| std::env::var("ORB_LOGO_T").ok()?.trim().parse().ok())
}

/// The animation time to draw now for an animation started at `start` (egui time): None when
/// it is over or off (the final state), else the seconds since the start. A frozen time wins.
pub fn time_now(now: f64, start: Option<f64>) -> Option<f32> {
    let t = match (frozen_time(), start) {
        (Some(t), _) => t,
        (None, Some(s)) => (now - s) as f32,
        (None, None) => return None,
    };
    (t < END_S).then_some(t.max(0.0))
}

/// One piece (or the whole part) at `alpha`, through the app's capped part painter
/// (`app::paint_part_tinted`, the texture-size cap of task-119).
fn paint_piece(ui: &Ui, name: &str, rect: Rect, alpha: f32) {
    if alpha > 0.0 {
        app::paint_part_tinted(ui, name, rect, Color32::WHITE.gamma_multiply(alpha));
    }
}

/// Paints the logo into `rect` (the logo canvas scaled): the whole part when `t` is None (the
/// final state), else the pieces at `t` seconds, and asks for the next frame.
pub fn paint_logo(ui: &mut Ui, rect: Rect, t: Option<f32>) {
    let Some(t) = t else {
        paint_piece(ui, LOGO_PART, rect, 1.0);
        return;
    };
    if frozen_time().is_none() {
        ui.ctx().request_repaint();
    }
    let k = rect.width() / LOGO_W;
    let f = Frame::at(t);
    paint_piece(ui, "logo_rb338", rect, 1.0);
    // OPEN, masked to the left of the R
    let old = ui.clip_rect();
    let mask = Rect::from_min_max(rect.min, egui::pos2(rect.min.x + RB_X * k, rect.max.y));
    ui.set_clip_rect(old.intersect(mask));
    paint_piece(
        ui,
        "logo_open",
        rect.translate(egui::vec2(f.open_dx * k, 0.0)),
        1.0,
    );
    ui.set_clip_rect(old);
    for (i, a) in f.seg.iter().enumerate() {
        paint_piece(ui, &format!("logo_line_{i}"), rect, *a);
    }
    paint_piece(ui, "logo_sub", rect, f.sub);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    /// The travel is OPEN's width and the gap, as the art lays the wordmark out.
    #[test]
    fn travel_matches_the_art() {
        let want = orb_ui_art::font::text_width(14.0, "OPEN") + orb_ui_art::font::gap(14.0);
        assert!(
            (OPEN_TRAVEL - want).abs() < 0.001,
            "{OPEN_TRAVEL} vs {want}"
        );
        assert_eq!(orb_ui_art::parts::LOGO_W, LOGO_W);
        assert_eq!(orb_ui_art::parts::LOGO_H, LOGO_H);
    }

    /// Runs the panel for a few frames at egui times from `t0` (so the SVGs are loaded) and
    /// returns the shapes of the last one.
    fn painted(ctx: &egui::Context, app: &mut crate::app::App, t0: f64) -> String {
        let mut last = String::new();
        for i in 0..6 {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(crate::layout::PANEL_W, crate::layout::PANEL_H),
                )),
                time: Some(t0 + 0.016 * i as f64),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| app.show(ui));
            // the ids of the managed textures differ between runs: compare the rest
            last = format!("{:?}", out.shapes);
            while let Some(p) = last.find("Managed(") {
                let end = p + last[p..].find(')').unwrap() + 1;
                last.replace_range(p..end, "Texture");
            }
            out.textures_delta.clear();
        }
        last
    }

    /// After the animation the panel paints exactly what it paints with no animation (the
    /// static wordmark: the same shapes), and in the middle it paints something else.
    #[test]
    fn the_end_state_paints_the_static_wordmark() {
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        let mut still = crate::app::App::new(crate::model::Panel::new(), None);
        let want = painted(&ctx, &mut still, 100.0);
        let mut anim = crate::app::App::new(crate::model::Panel::new(), None);
        anim.logo_anim = true;
        anim.logo_start = Some(100.0);
        if frozen_time().is_none() {
            let mid = painted(&ctx, &mut anim, 100.5);
            assert_ne!(mid, want, "mid-animation differs from the static logo");
            let got = painted(&ctx, &mut anim, 100.0 + END_S as f64 + 0.1);
            let i = got
                .bytes()
                .zip(want.bytes())
                .take_while(|(a, b)| a == b)
                .count();
            let lo = i.saturating_sub(150);
            assert!(
                got == want,
                "differ at {i} of {}/{}: {:?} vs {:?}",
                got.len(),
                want.len(),
                &got[lo..(i + 150).min(got.len())],
                &want[lo..(i + 150).min(want.len())]
            );
        }
    }

    #[test]
    fn the_start() {
        let f = Frame::at(0.0);
        assert!(close(f.open_dx, OPEN_TRAVEL));
        assert_eq!(f.seg, [0.0; 4]);
        assert_eq!((f.sub, f.icon), (0.0, 0.0));
    }

    #[test]
    fn the_slide_eases_out() {
        // ease-out cubic: 1 - (1 - t)^3 of the way at t seconds of 1
        assert!(close(Frame::at(0.5).open_dx, OPEN_TRAVEL * 0.125));
        assert!(close(Frame::at(0.25).open_dx, OPEN_TRAVEL * 0.421875));
        assert!(close(Frame::at(1.0).open_dx, 0.0));
        assert!(Frame::at(0.1).open_dx > Frame::at(0.2).open_dx);
    }

    #[test]
    fn the_underline_fades_left_to_right() {
        assert_eq!(Frame::at(0.55).seg[0], 0.0);
        assert!(close(Frame::at(0.675).seg[0], 0.5));
        assert!(close(Frame::at(0.8).seg[0], 1.0));
        assert!(close(Frame::at(0.875).seg[1], 0.5));
        assert!(close(Frame::at(1.075).seg[2], 0.5));
        assert!(close(Frame::at(1.275).seg[3], 0.5));
        assert_eq!(Frame::at(0.7).seg[1], 0.0);
        assert!(close(Frame::at(1.4).seg[3], 1.0));
    }

    #[test]
    fn the_subtitle_and_the_icon_fade() {
        assert_eq!(Frame::at(1.35).sub, 0.0);
        assert!(close(Frame::at(1.6).sub, 0.5));
        assert!(close(Frame::at(1.85).sub, 1.0));
        assert!(close(Frame::at(0.3).icon, 0.5));
        assert!(close(Frame::at(0.6).icon, 1.0));
    }

    /// At the end every piece is in its final state, and nothing changes after it.
    #[test]
    fn the_final_state() {
        let end = Frame::at(END_S);
        assert_eq!(end.open_dx, 0.0);
        assert_eq!(end.seg, [1.0; 4]);
        assert_eq!((end.sub, end.icon), (1.0, 1.0));
        assert_eq!(Frame::at(10.0), end);
        // the clock: over (the whole part is painted) at END_S, none without a start
        assert_eq!(time_now(5.0, None), None);
        if frozen_time().is_none() {
            assert_eq!(time_now(5.0 + END_S as f64, Some(5.0)), None);
            assert!(time_now(5.5, Some(5.0)).is_some());
        }
    }
}
