//! task-116: the wordmark is its own part (and its pieces: OPEN, RB-338, the four underline
//! segments of task-115 and the subtitle), drawn once over the transport background, which
//! holds no logo of its own.

use orb_ui_art::layout::layout;
use orb_ui_art::parts::{LOGO_H, LOGO_PIECES, LOGO_W, LOGO_X, LOGO_Y, all_parts};
use orb_ui_art::svg::pal;

#[test]
fn the_logo_is_its_own_part() {
    let parts = all_parts();
    let get = |name: &str| parts.iter().find(|p| p.name == name).expect(name);
    let logo = get("logo");
    assert_eq!((logo.w, logo.h), (LOGO_W, LOGO_H));
    // the whole is the pieces, in order, each on the same canvas
    let mut joined = String::new();
    for name in LOGO_PIECES {
        let p = get(name);
        assert_eq!((p.w, p.h), (LOGO_W, LOGO_H), "{name}");
        assert!(!p.body.is_empty(), "{name}");
        joined += &p.body;
    }
    assert_eq!(logo.body, joined);
    // the INK and ORANGE texts and the orange underline are in it
    assert!(logo.body.contains(pal::ORANGE) && logo.body.contains(pal::INK));
    // the transport background has none of it
    assert!(
        !get("bg_transport").body.contains(pal::ORANGE),
        "bg_transport holds no wordmark"
    );
    // the logo is drawn after the transport background, at the click area of the panel
    let l = layout(&parts);
    let pos = |n: &str| l.backgrounds.iter().position(|b| b.0 == n).expect(n);
    assert!(pos("bg_transport") < pos("logo"));
    let b = &l.backgrounds[pos("logo")];
    assert_eq!((b.1, b.2), (LOGO_X, LOGO_Y));
    // (this fork: 7 px left of the original's 608, about 10 px from the transport's edge)
    assert_eq!((LOGO_X + LOGO_W, LOGO_Y + LOGO_H), (601.0, 49.0));
}

/// The underline's segments (x, width): four, as long as the wordmark `OpenRB-338x`, 1.5 px
/// apart, from the wordmark's left edge (this fork's wordmark; task-115 had 28.4 px at 29.9).
fn segments() -> [(f32, f32); 4] {
    use orb_ui_art::font::{gap, text_width};
    use orb_ui_art::parts::{LOGO_CAP, LOGO_TEXT_X};
    let (cap, x0) = (LOGO_CAP, LOGO_TEXT_X);
    // summed in the order the logo lays the wordmark out, so the floats agree to the last bit
    let x1 = x0 + text_width(cap, "OpenRB-") + gap(cap);
    let x2 = x1 + text_width(cap, "338") + gap(cap);
    // to the outer edge of the x's round stroke
    let word = x2 + text_width(cap, "x") + orb_ui_art::font::stroke(cap) / 2.0 - x0;
    let w = (word - 3.0 * 1.5) / 4.0;
    std::array::from_fn(|i| (LOGO_TEXT_X + (w + 1.5) * i as f32, w))
}

/// The wordmark drawn as one run: one INK run 'OpenRB-', the ORANGE '338', the WHITE 'x', the
/// underline and the subtitle (as task-068 drew it into `bg_transport`, in this fork's form).
fn wordmark_as_one_run() -> String {
    use orb_ui_art::font::{Anchor, gap, text, text_width};
    use orb_ui_art::parts::{LOGO_CAP, LOGO_TEXT_X};
    use orb_ui_art::svg::Svg;
    let mut s = Svg::new();
    let (cap, x0) = (LOGO_CAP, LOGO_TEXT_X);
    text(&mut s, x0, 17.0, cap, Anchor::Start, pal::INK, "OpenRB-");
    let x1 = x0 + text_width(cap, "OpenRB-") + gap(cap);
    text(&mut s, x1, 17.0, cap, Anchor::Start, pal::ORANGE, "338");
    let x2 = x1 + text_width(cap, "338") + gap(cap);
    text(&mut s, x2, 17.0, cap, Anchor::Start, pal::WHITE, "x");
    // the underline: four segments in the 808's step colors
    for ((x, w), color) in segments()
        .into_iter()
        .zip([pal::RED, pal::ORANGE, pal::YELLOW, pal::WHITE])
    {
        s.rect(x, 28.0, w, 1.5, 0.75, color);
    }
    let sub = "BASS LINE AND DRUM MACHINE";
    text(&mut s, x0, 37.0, 3.4, Anchor::Start, pal::INK_DIM, sub);
    s.s
}

/// The split into pieces changes no pixel: the logo part and the old single-run drawing render
/// the same at 1x, 2x and 3x.
#[test]
fn the_logo_pieces_render_like_the_single_run() {
    use orb_ui_art::render;
    let parts = all_parts();
    let svg = |body: &str| {
        format!(
            concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="134" height="48" "#,
                r#"viewBox="0 0 134 48">{body}</svg>"#
            ),
            body = body
        )
    };
    let whole = parts.iter().find(|p| p.name == "logo").unwrap();
    for scale in 1..=3 {
        let a = render(&svg(&wordmark_as_one_run()), scale).unwrap();
        let b = render(&whole.to_svg(), scale).unwrap();
        assert_eq!(a.data(), b.data(), "scale {scale}");
        assert!(a.data().iter().any(|&v| v != 0), "something is drawn");
    }
}

/// task-115: the underline is four separate pieces, one per segment, each in one step color of
/// the 808, so an animation can fade them in one by one; together as long as the wordmark,
/// which still fits in the logo part.
#[test]
fn the_underline_is_four_colored_segments() {
    let parts = all_parts();
    let colors = [pal::RED, pal::ORANGE, pal::YELLOW, pal::WHITE];
    let segs = segments();
    for (i, color) in colors.into_iter().enumerate() {
        let name = format!("logo_line_{i}");
        let p = parts.iter().find(|p| p.name == name).expect(&name);
        let (x, w) = segs[i];
        let want = format!(
            concat!(
                "<rect x=\"{}\" y=\"28\" width=\"{}\" height=\"1.5\" rx=\"0.75\" ",
                "fill=\"{}\"/>"
            ),
            orb_ui_art::svg::n(x),
            orb_ui_art::svg::n(w),
            color
        );
        assert_eq!(p.body, want, "{name}");
    }
    let (last_x, last_w) = segs[3];
    assert!(last_x + last_w < LOGO_W, "the wordmark fits in the logo");
    assert!(parts.iter().all(|p| p.name != "logo_line"));
}
