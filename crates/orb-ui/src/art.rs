//! The panel's SVG parts (`assets/ui`), embedded by `build.rs`, and the knob designs.

include!(concat!(env!("OUT_DIR"), "/parts_gen.rs"));

/// A part: its URI for egui's loaders, its SVG bytes and its bleed: how far, in panel pixels,
/// the art reaches beyond its control's rectangle on every side (shadows, halos, glows;
/// task-179). The SVG's canvas is the rectangle grown by the bleed.
#[derive(Clone, Copy)]
pub struct Part {
    pub name: &'static str,
    pub uri: &'static str,
    pub svg: &'static [u8],
    pub bleed: f32,
    /// The control's rectangle (panel pixels) the part is drawn for.
    pub w: f32,
    pub h: f32,
}

impl Part {
    fn from_row(row: &PartRow) -> Self {
        let &(name, uri, svg, bleed, w, h) = row;
        Self {
            name,
            uri,
            svg,
            bleed,
            w,
            h,
        }
    }
}

/// The part called `name` (`PARTS` is sorted by name).
pub fn part(name: &str) -> Option<Part> {
    let i = PARTS.binary_search_by(|p| p.0.cmp(name)).ok()?;
    Some(Part::from_row(&PARTS[i]))
}

/// The part names asked for that no art has, in this process.
static MISSING: std::sync::Mutex<std::collections::BTreeSet<String>> =
    std::sync::Mutex::new(std::collections::BTreeSet::new());

/// Records that the part `name` is missing; true the first time it is.
pub fn report_missing(name: &str) -> bool {
    let mut m = MISSING.lock().unwrap_or_else(|e| e.into_inner());
    m.insert(name.to_owned())
}

/// The part names drawn as nothing because no art has them, in this process (task-156).
pub fn missing() -> Vec<String> {
    let m = MISSING.lock().unwrap_or_else(|e| e.into_inner());
    m.iter().cloned().collect()
}

/// Every part.
pub fn all() -> impl Iterator<Item = Part> {
    PARTS.iter().map(Part::from_row)
}

/// A knob design: the layers are `<name>_body` (static) and `<name>_pointer`, rotated about
/// `(cx, cy)` (assets/ui/README.md). The knobs have no value arc since task-179.
pub struct KnobDesign {
    pub name: &'static str,
    pub w: f32,
    pub h: f32,
    pub cx: f32,
    pub cy: f32,
}

pub const KNOBS: [KnobDesign; 5] = [
    KnobDesign {
        name: "knob_303",
        w: 30.0,
        h: 30.0,
        cx: 15.0,
        cy: 15.0,
    },
    KnobDesign {
        name: "knob_808_red",
        w: 13.0,
        h: 13.0,
        cx: 6.5,
        cy: 6.5,
    },
    KnobDesign {
        name: "knob_808_white",
        w: 13.0,
        h: 13.0,
        cx: 6.5,
        cy: 6.5,
    },
    KnobDesign {
        name: "knob_909",
        w: 12.0,
        h: 12.0,
        cx: 6.0,
        cy: 6.0,
    },
    KnobDesign {
        name: "knob_gray",
        w: 18.0,
        h: 19.0,
        cx: 9.0,
        cy: 9.5,
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
        assert_eq!(KNOBS.len(), orb_ui_art::parts::KNOBS.len());
        for (a, b) in KNOBS.iter().zip(orb_ui_art::parts::KNOBS.iter()) {
            assert_eq!(a.name, b.name);
            assert_eq!((a.w, a.h, a.cx, a.cy), (b.w, b.h, b.cx, b.cy));
            // the pointer turns about the canvas's center: the art centers the knob
            assert_eq!((a.cx * 2.0, a.cy * 2.0), (a.w, a.h), "{}", a.name);
        }
    }

    #[test]
    fn all_generated_parts_are_embedded_and_sorted() {
        let generated = orb_ui_art::parts::all_parts();
        assert_eq!(PARTS.len(), generated.len());
        assert!(PARTS.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(part("btn_play_lit").is_some());
        assert!(part("panel").is_none());
        assert!(part("nope").is_none());
        // the bleed read from each file is the generator's
        for g in generated {
            let p = part(&g.name).unwrap();
            assert_eq!(p.bleed, g.bleed, "{}", g.name);
            assert!(
                (p.w - g.w).abs() < 1e-4 && (p.h - g.h).abs() < 1e-4,
                "{}",
                g.name
            );
        }
    }
}
