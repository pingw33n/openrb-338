//! task-116: the wordmark is its own part (and four pieces), drawn once over the transport
//! background, which holds no logo of its own.

use rb_ui_art::layout::layout;
use rb_ui_art::parts::{LOGO_H, LOGO_PIECES, LOGO_W, LOGO_X, LOGO_Y, all_parts};
use rb_ui_art::svg::pal;

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
    assert_eq!((LOGO_X + LOGO_W, LOGO_Y + LOGO_H), (608.0, 49.0));
}

/// The wordmark as task-068 drew it into `bg_transport`: one INK run 'OPENRB-' and the rest.
fn wordmark_as_one_run() -> String {
    use rb_ui_art::font::{Anchor, gap, text, text_width};
    use rb_ui_art::svg::Svg;
    let mut s = Svg::new();
    let cap = 14.0;
    text(&mut s, 8.0, 17.0, cap, Anchor::Start, pal::INK, "OPENRB-");
    let x1 = 8.0 + text_width(cap, "OPENRB-") + gap(cap);
    text(&mut s, x1, 17.0, cap, Anchor::Start, pal::ORANGE, "338");
    s.rect(8.0, 28.0, 118.0, 1.5, 0.75, pal::ORANGE);
    let sub = "BASS LINE AND DRUM MACHINE";
    text(&mut s, 8.0, 37.0, 3.4, Anchor::Start, pal::INK_DIM, sub);
    s.s
}

/// The split into pieces changes no pixel: the logo part and the old single-run drawing render
/// the same at 1x, 2x and 3x.
#[test]
fn the_logo_pieces_render_like_the_single_run() {
    use rb_ui_art::render;
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
