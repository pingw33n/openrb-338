//! The labels of the panel in Inter 4.1 (task-179), turned into outlines when the art is
//! generated, so the written SVGs hold no `<text>` and the app needs no font.
//!
//! usvg lays the text out and converts it to paths with only the three TTFs of
//! `crates/orb-ui-art/fonts` in its font database (no system font), so the outlines are the
//! same on every machine. Inter is under the SIL Open Font License 1.1 (`fonts/LICENSE.txt`).

use std::collections::HashMap;
use std::fmt::Write;
use std::sync::{Arc, Mutex, OnceLock};

use resvg::tiny_skia::{Path, PathSegment};
use resvg::usvg::{self, Node, fontdb};

use crate::svg::{Svg, n};

const REGULAR: &[u8] = include_bytes!("../fonts/Inter-Regular.ttf");
const MEDIUM: &[u8] = include_bytes!("../fonts/Inter-Medium.ttf");
const BOLD: &[u8] = include_bytes!("../fonts/Inter-Bold.ttf");

/// The font database: the three bundled weights of Inter and nothing else.
fn fonts() -> Arc<fontdb::Database> {
    static DB: OnceLock<Arc<fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = fontdb::Database::new();
        for data in [REGULAR, MEDIUM, BOLD] {
            db.load_font_data(data.to_vec());
        }
        Arc::new(db)
    })
    .clone()
}

/// Where a text's x is: its start, its middle or its end (SVG's `text-anchor`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Anchor {
    Start,
    Middle,
    End,
}

impl Anchor {
    fn svg(self) -> &'static str {
        match self {
            Anchor::Start => "start",
            Anchor::Middle => "middle",
            Anchor::End => "end",
        }
    }
}

/// A label's layout: the baseline point, the text, the size, the anchor, the weight (400, 500
/// or 700) and the letter spacing.
#[derive(Clone, PartialEq, Debug)]
pub struct Label<'a> {
    pub x: f32,
    pub y: f32,
    pub text: &'a str,
    pub size: f32,
    pub anchor: Anchor,
    pub weight: u16,
    pub spacing: f32,
}

fn append(out: &mut String, path: &Path) {
    for seg in path.segments() {
        match seg {
            PathSegment::MoveTo(p) => {
                let _ = write!(out, "M{} {}", n(p.x), n(p.y));
            }
            PathSegment::LineTo(p) => {
                let _ = write!(out, "L{} {}", n(p.x), n(p.y));
            }
            PathSegment::QuadTo(a, p) => {
                let _ = write!(out, "Q{} {} {} {}", n(a.x), n(a.y), n(p.x), n(p.y));
            }
            PathSegment::CubicTo(a, b, p) => {
                let _ = write!(
                    out,
                    "C{} {} {} {} {} {}",
                    n(a.x),
                    n(a.y),
                    n(b.x),
                    n(b.y),
                    n(p.x),
                    n(p.y)
                );
            }
            PathSegment::Close => out.push('Z'),
        }
    }
}

fn collect(g: &usvg::Group, out: &mut String) {
    for node in g.children() {
        match node {
            Node::Group(g) => collect(g, out),
            Node::Path(p) => {
                if let Some(path) = p.data().clone().transform(p.abs_transform()) {
                    append(out, &path);
                }
            }
            Node::Text(t) => collect(t.flattened(), out),
            Node::Image(_) => {}
        }
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;")
}

/// The outline of a label as SVG path data, in the coordinates the label is given in.
pub fn outline(l: &Label) -> String {
    type Key = (String, [u32; 4], Anchor, u16);
    static CACHE: OnceLock<Mutex<HashMap<Key, String>>> = OnceLock::new();
    let key = (
        l.text.to_string(),
        [l.x, l.y, l.size, l.spacing].map(f32::to_bits),
        l.anchor,
        l.weight,
    );
    let cache = CACHE.get_or_init(Default::default);
    if let Some(d) = cache.lock().unwrap().get(&key) {
        return d.clone();
    }
    let svg = format!(
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="1000" height="1000">"#,
            r#"<text x="{}" y="{}" font-family="Inter" font-size="{}" font-weight="{}" "#,
            r#"text-anchor="{}" letter-spacing="{}">{}</text></svg>"#
        ),
        n(l.x),
        n(l.y),
        n(l.size),
        l.weight,
        l.anchor.svg(),
        n(l.spacing),
        escape(l.text)
    );
    let opt = usvg::Options {
        fontdb: fonts(),
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(&svg, &opt).expect("a label parses");
    let mut d = String::new();
    collect(tree.root(), &mut d);
    assert!(
        !d.is_empty() || l.text.trim().is_empty(),
        "no outline for {:?}: Inter is not in the font database",
        l.text
    );
    cache.lock().unwrap().insert(key, d.clone());
    d
}

/// Draws a label as a filled path; `extra` is added to the element (an opacity, a filter).
pub fn draw(s: &mut Svg, l: &Label, fill: &str, extra: &str) {
    let d = outline(l);
    if !d.is_empty() {
        let _ = write!(s.s, r#"<path d="{d}" fill="{fill}" {extra}/>"#);
    }
}
