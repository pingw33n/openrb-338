//! task-116: the wordmark is its own part (and its pieces: OPEN, RB-338, the four underline
//! segments of task-115 and the subtitle), drawn once over the transport background, which
//! holds no logo of its own. task-179 restyled the pieces (embossed strokes, glossy bars);
//! the glyph layout and the pieces stayed.

use orb_ui_art::layout::layout;
use orb_ui_art::parts::{LOGO_H, LOGO_PIECES, LOGO_W, LOGO_X, LOGO_Y, all_parts};

/// The ink of OPENRB- and of 338.
const INK: &str = "#f2f5f7";
const ORANGE: &str = "#ff9a2e";

#[test]
fn the_logo_is_its_own_part() {
    let parts = all_parts();
    let get = |name: &str| parts.iter().find(|p| p.name == name).expect(name);
    let logo = get("logo");
    assert_eq!((logo.w, logo.h, logo.bleed), (LOGO_W, LOGO_H, 0.0));
    // the whole is the pieces, in order, each on the same canvas
    let mut joined = String::new();
    for name in LOGO_PIECES {
        let p = get(name);
        assert_eq!((p.w, p.h, p.bleed), (LOGO_W, LOGO_H, 0.0), "{name}");
        assert!(!p.body.is_empty(), "{name}");
        joined += &p.body;
    }
    assert_eq!(logo.body, joined);
    assert!(logo.body.contains(ORANGE) && logo.body.contains(INK));
    // the transport background has none of it
    assert!(
        !get("bg_transport").body.contains(ORANGE),
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

/// The glyphs are laid out as the stroke font lays the word out (cap 14, so 2.33 glyph units
/// per pixel), the run at x 8 and 338 right after `OPENRB-` and a gap: the travel of OPEN in
/// the startup animation (`logo_anim::OPEN_TRAVEL`) depends on it.
#[test]
fn the_glyphs_keep_the_stroke_fonts_layout() {
    use orb_ui_art::font::{gap, text_width};
    let parts = all_parts();
    let body = |name: &str| parts.iter().find(|p| p.name == name).unwrap().body.clone();
    let open = body("logo_open");
    for dx in ["0", "5.4", "10.8", "15.8"] {
        assert!(
            open.contains(&format!("translate({dx})")),
            "O P E N at {dx}"
        );
    }
    assert!(open.contains("translate(8 9.5) scale(2.33)"));
    let x338 = 8.0 + text_width(14.0, "OPENRB-") + gap(14.0);
    assert!((x338 - 92.0).abs() < 0.01, "{x338}");
    let rb = body("logo_rb338");
    assert!(rb.contains("translate(92 9.5) scale(2.33)") && rb.contains(ORANGE));
    assert!(rb.contains("translate(21.2)") && rb.contains("translate(32)"));
}

/// task-115: the underline is four separate pieces, one per segment, each in one step color of
/// the 808 (task-179: as glossy as the step keys) and 28.4 px wide at 29.9 px steps, so an
/// animation can fade them in one by one.
#[test]
fn the_underline_is_four_colored_segments() {
    let parts = all_parts();
    for (i, c) in ["R", "O", "Y", "W"].into_iter().enumerate() {
        let name = format!("logo_line_{i}");
        let p = parts.iter().find(|p| p.name == name).expect(&name);
        let x = orb_ui_art::svg::n(8.0 + 29.9 * i as f32);
        let want = format!(
            r#"<rect x="{x}" y="27.6" width="28.4" height="2.6" rx="0.9" fill="url(#g808k{c})""#
        );
        assert!(p.body.contains(&want), "{name}: {}", p.body);
        assert!(
            p.to_svg().contains(&format!("id=\"g808k{c}\"")),
            "{name} carries its defs"
        );
    }
    assert!(parts.iter().all(|p| p.name != "logo_line"));
}
