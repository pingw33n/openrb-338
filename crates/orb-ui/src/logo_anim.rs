//! The startup animation of the wordmark OpenRB-338x (task-114, redone for this fork): the
//! pieces appear one after another, each fading in:
//!
//! - `OpenRB-` over [`OPENRB_S`], from 0;
//! - `338` over [`N338_S`], right after it;
//! - the four underline segments, red, orange, yellow, white, each over [`SEG_FADE_S`], from
//!   [`SEG_START_S`] on, [`SEG_STEP_S`] apart;
//! - last the `x`, above the white segment, over [`X_FADE_S`] from [`X_START_S`], with a soft
//!   flash: a glow around it that rises with it and dies away by [`FLASH_END_S`];
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

/// `OpenRB-`'s fade, from 0, and `338`'s, right after it.
pub const OPENRB_S: f32 = 0.35;
pub const N338_S: f32 = 0.35;
/// The first underline segment's start, the step to the next, and each one's fade.
pub const SEG_START_S: f32 = 0.75;
pub const SEG_STEP_S: f32 = 0.20;
pub const SEG_FADE_S: f32 = 0.25;
/// The x: its start (after the white segment), its fade, and the end of its flash.
pub const X_START_S: f32 = 1.65;
pub const X_FADE_S: f32 = 0.15;
pub const FLASH_END_S: f32 = 2.25;
/// The subtitle's fade.
pub const SUB_START_S: f32 = 1.9;
pub const SUB_END_S: f32 = 2.3;
/// The About icon's fade, from 0.
pub const ICON_FADE_S: f32 = 0.6;
/// The end of the whole animation.
pub const END_S: f32 = SUB_END_S;

/// The whole logo part and its size (a test checks them against orb-ui-art's).
pub const LOGO_PART: &str = "logo";
pub const LOGO_W: f32 = 134.0;
pub const LOGO_H: f32 = 48.0;
/// The center of the x in the logo part, where it flashes (orb-ui-art's `logo_x_pos`; a test
/// checks it).
pub const X_CENTER: (f32, f32) = (127.033_3, 17.0);

/// The animation's state at one moment: each piece's opacity, and the x's flash (0 to 1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub openrb: f32,
    pub n338: f32,
    /// The underline segments' opacities, left to right.
    pub seg: [f32; 4],
    pub x: f32,
    pub flash: f32,
    pub sub: f32,
    pub icon: f32,
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
        // the flash rises with the x and fades out after it, eased so it stays soft
        let peak = X_START_S + X_FADE_S;
        let flash = if t < peak {
            ramp(t, X_START_S, X_FADE_S)
        } else {
            let k = 1.0 - ramp(t, peak, FLASH_END_S - peak);
            k * k
        };
        Self {
            openrb: ramp(t, 0.0, OPENRB_S),
            n338: ramp(t, OPENRB_S, N338_S),
            seg,
            x: ramp(t, X_START_S, X_FADE_S),
            flash,
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
    paint_piece(ui, "logo_openrb", rect, f.openrb);
    paint_piece(ui, "logo_338", rect, f.n338);
    for (i, a) in f.seg.iter().enumerate() {
        paint_piece(ui, &format!("logo_line_{i}"), rect, *a);
    }
    if f.flash > 0.0 {
        // a soft glow behind the x, and the x itself a little larger and fainter over it
        let c = rect.min + egui::vec2(X_CENTER.0, X_CENTER.1) * k;
        let p = ui.painter();
        for (r, a) in [(11.0, 0.10), (7.0, 0.16), (4.5, 0.22)] {
            p.circle_filled(c, r * k, Color32::WHITE.gamma_multiply(a * f.flash));
        }
        let grow = 1.0 + 0.18 * f.flash;
        let big = Rect::from_min_size(
            c + (rect.min - c) * grow,
            rect.size() * grow,
        );
        paint_piece(ui, "logo_x", big, 0.5 * f.flash);
    }
    paint_piece(ui, "logo_x", rect, f.x);
    paint_piece(ui, "logo_sub", rect, f.sub);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    /// The flash's center is the x's center, as the art lays the wordmark out.
    #[test]
    fn the_flash_is_on_the_x() {
        let (_, cx) = orb_ui_art::parts::logo_x_pos();
        assert!((X_CENTER.0 - cx).abs() < 0.001, "{} vs {cx}", X_CENTER.0);
        assert_eq!(orb_ui_art::parts::LOGO_W, LOGO_W);
        assert_eq!(orb_ui_art::parts::LOGO_H, LOGO_H);
        assert_eq!(
            orb_ui_art::parts::LOGO_PIECES,
            [
                "logo_openrb",
                "logo_338",
                "logo_line_0",
                "logo_line_1",
                "logo_line_2",
                "logo_line_3",
                "logo_x",
                "logo_sub"
            ]
        );
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
        assert_eq!((f.openrb, f.n338, f.x, f.flash), (0.0, 0.0, 0.0, 0.0));
        assert_eq!(f.seg, [0.0; 4]);
        assert_eq!((f.sub, f.icon), (0.0, 0.0));
    }

    /// OpenRB-, then 338, then the segments red to white, then the x: each starts when the one
    /// before it is in.
    #[test]
    fn the_pieces_come_in_order() {
        assert!(close(Frame::at(0.175).openrb, 0.5));
        assert_eq!(Frame::at(0.35).n338, 0.0);
        assert!(close(Frame::at(0.35).openrb, 1.0));
        assert!(close(Frame::at(0.525).n338, 0.5));
        assert_eq!(Frame::at(0.75).seg[0], 0.0);
        assert!(close(Frame::at(0.875).seg[0], 0.5));
        assert!(close(Frame::at(1.075).seg[1], 0.5));
        assert!(close(Frame::at(1.275).seg[2], 0.5));
        assert!(close(Frame::at(1.475).seg[3], 0.5));
        assert!(close(Frame::at(1.6).seg[3], 1.0));
        assert_eq!(Frame::at(1.6).x, 0.0);
        assert!(close(Frame::at(1.725).x, 0.5));
        assert!(close(Frame::at(1.8).x, 1.0));
    }

    /// The x's flash rises with it, is full when the x is in, and dies away softly after.
    #[test]
    fn the_x_flashes_softly() {
        assert_eq!(Frame::at(1.6).flash, 0.0);
        assert!(close(Frame::at(1.725).flash, 0.5));
        assert!(close(Frame::at(1.8).flash, 1.0));
        let a = Frame::at(1.9).flash;
        let b = Frame::at(2.1).flash;
        assert!(a < 1.0 && b < a && b > 0.0, "{a} {b}");
        assert_eq!(Frame::at(FLASH_END_S).flash, 0.0);
    }

    #[test]
    fn the_subtitle_and_the_icon_fade() {
        assert_eq!(Frame::at(1.9).sub, 0.0);
        assert!(close(Frame::at(2.1).sub, 0.5));
        assert!(close(Frame::at(2.3).sub, 1.0));
        assert!(close(Frame::at(0.3).icon, 0.5));
        assert!(close(Frame::at(0.6).icon, 1.0));
    }

    /// At the end every piece is in its final state, and nothing changes after it.
    #[test]
    fn the_final_state() {
        let end = Frame::at(END_S);
        assert_eq!((end.openrb, end.n338, end.x, end.flash), (1.0, 1.0, 1.0, 0.0));
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
