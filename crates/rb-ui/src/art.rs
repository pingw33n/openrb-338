//! The panel's SVG parts (`assets/ui`), embedded by `build.rs`, and the knob designs.

include!(concat!(env!("OUT_DIR"), "/parts_gen.rs"));

/// A part: its URI for egui's loaders and its SVG bytes.
#[derive(Clone, Copy)]
pub struct Part {
    pub name: &'static str,
    pub uri: &'static str,
    pub svg: &'static [u8],
}

/// The part called `name` (`PARTS` is sorted by name).
pub fn part(name: &str) -> Option<Part> {
    let i = PARTS.binary_search_by(|p| p.0.cmp(name)).ok()?;
    let (name, uri, svg) = PARTS[i];
    Some(Part { name, uri, svg })
}

/// Every part.
pub fn all() -> impl Iterator<Item = Part> {
    PARTS
        .iter()
        .map(|&(name, uri, svg)| Part { name, uri, svg })
}

/// A knob design: the layers are `<name>_body`, the arc painted by the app and
/// `<name>_pointer` rotated about `(cx, cy)` (assets/ui/README.md).
pub struct KnobDesign {
    pub name: &'static str,
    pub w: f32,
    pub h: f32,
    pub cx: f32,
    pub cy: f32,
    /// The value arc's radius and width, 0 for none.
    pub ring_r: f32,
    pub ring_w: f32,
    /// The arc color as RGB.
    pub arc: [u8; 3],
}

pub const KNOBS: [KnobDesign; 5] = [
    KnobDesign {
        name: "knob_303",
        w: 30.0,
        h: 30.0,
        cx: 15.0,
        cy: 15.0,
        ring_r: 13.6,
        ring_w: 1.6,
        arc: [0xf0, 0x8a, 0x24],
    },
    KnobDesign {
        name: "knob_808_red",
        w: 13.0,
        h: 13.0,
        cx: 6.5,
        cy: 6.5,
        ring_r: 5.9,
        ring_w: 0.9,
        arc: [0xf2, 0xc5, 0x31],
    },
    KnobDesign {
        name: "knob_808_white",
        w: 13.0,
        h: 13.0,
        cx: 6.5,
        cy: 6.5,
        ring_r: 5.9,
        ring_w: 0.9,
        arc: [0xf0, 0x8a, 0x24],
    },
    KnobDesign {
        name: "knob_909",
        w: 12.0,
        h: 12.0,
        cx: 6.0,
        cy: 6.0,
        ring_r: 5.4,
        ring_w: 0.9,
        arc: [0xe2, 0x41, 0x3b],
    },
    KnobDesign {
        name: "knob_22",
        w: 22.0,
        h: 22.0,
        cx: 11.0,
        cy: 11.0,
        ring_r: 10.1,
        ring_w: 1.4,
        arc: [0xff, 0xb0, 0x00],
    },
];

/// The design of a knob control, by its layout part name.
pub fn knob_design(part: &str) -> &'static KnobDesign {
    KNOBS
        .iter()
        .find(|k| k.name == part)
        .unwrap_or_else(|| panic!("no knob design {part}"))
}

/// The window's icon: the `app_icon` part rendered at `size` x `size` pixels.
pub fn app_icon(size: u32) -> eframe::egui::IconData {
    let svg = part("app_icon").expect("app_icon part").svg;
    let tree = resvg::usvg::Tree::from_data(svg, &resvg::usvg::Options::default())
        .expect("app_icon parses");
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).unwrap();
    let scale = size as f32 / tree.size().width();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    // IconData is straight (not premultiplied) RGBA.
    let rgba = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    eframe::egui::IconData {
        rgba,
        width: size,
        height: size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knob_designs_match_the_art_generator() {
        for (a, b) in KNOBS.iter().zip(rb_ui_art::parts::KNOBS.iter()) {
            assert_eq!(a.name, b.name);
            assert_eq!((a.w, a.h, a.cx, a.cy), (b.w, b.h, b.cx, b.cy));
            assert_eq!((a.ring_r, a.ring_w), (b.ring_r, b.ring_w));
            let hex = format!("#{:02x}{:02x}{:02x}", a.arc[0], a.arc[1], a.arc[2]);
            assert_eq!(hex, b.arc, "{}", a.name);
        }
    }

    #[test]
    fn all_generated_parts_are_embedded_and_sorted() {
        assert_eq!(PARTS.len(), rb_ui_art::parts::all_parts().len());
        assert!(PARTS.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(part("btn_play_lit").is_some());
        assert!(part("panel").is_none());
        assert!(part("nope").is_none());
    }
}
