//! Where the controls and the section backgrounds sit, in panel pixels
//! (`assets/ui/layout.tsv` and the README's background table).

pub const PANEL_W: f32 = 624.0;
pub const PANEL_H: f32 = 510.0;

const LAYOUT_TSV: &str = include_str!("../../../assets/ui/layout.tsv");

/// A control's rectangle and the part that draws its default state.
#[derive(Clone, Debug)]
pub struct Ctl {
    pub id: String,
    pub section: String,
    pub kind: String,
    pub part: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Ctl {
    /// The part's name without its state: `btn_play_lit` -> `btn_play_`. Not for knobs,
    /// faders and displays, whose parts are composed.
    pub fn family(&self) -> &str {
        match self.part.rfind('_') {
            Some(i) => &self.part[..=i],
            None => &self.part,
        }
    }

    /// The control's name within its section (`tb303_1.cutoff` -> `cutoff`).
    pub fn name(&self) -> &str {
        self.id.split_once('.').map_or(&self.id, |(_, n)| n)
    }
}

/// Parses `layout.tsv`.
pub fn controls() -> Vec<Ctl> {
    LAYOUT_TSV
        .lines()
        .skip(1)
        .map(|line| {
            let f: Vec<&str> = line.split('\t').collect();
            let n = |i: usize| f[i].parse::<f32>().expect("layout.tsv number");
            Ctl {
                id: f[0].into(),
                section: f[1].into(),
                kind: f[2].into(),
                part: f[3].into(),
                x: n(4),
                y: n(5),
                w: n(6),
                h: n(7),
            }
        })
        .collect()
}

/// The section backgrounds drawn first: (part, x, y). The row focus bars are not here.
pub fn backgrounds() -> Vec<(&'static str, f32, f32)> {
    let mut v = vec![
        ("wood", 0.0, 0.0),
        ("wood", 608.0, 0.0),
        ("bg_transport", 16.0, 0.0),
    ];
    for r in 0..4 {
        let y = 50.0 + 115.0 * r as f32;
        v.push(("bg_pattern", 16.0, y));
        v.push((
            match r {
                0 | 1 => "bg_tb303",
                2 => "bg_tr808",
                _ => "bg_tr909",
            },
            90.0,
            y,
        ));
        v.push(("bg_mixer", 456.0, y));
    }
    v.push(("bg_master", 526.0, 50.0));
    v.push(("bg_pcf", 526.0, 148.0));
    v.push(("bg_delay", 526.0, 259.0));
    v.push(("bg_dist", 526.0, 352.0));
    v.push(("bg_comp", 526.0, 417.0));
    v
}

/// The focus bar of row `r` (0..=3): (x, y, w, h).
pub fn focus_bar(r: usize) -> (f32, f32, f32, f32) {
    (86.0, 50.0 + 115.0 * r as f32, 4.0, 115.0)
}

/// The row (0..=3) containing a panel point, as the window's click handler tests it.
pub fn row_at(x: f32, y: f32) -> Option<usize> {
    if !(16.0..526.0).contains(&x) || !(50.0..510.0).contains(&y) {
        return None;
    }
    Some(((y - 50.0) / 115.0) as usize)
}

/// Whether a panel point is on the logo, the hit rectangle of `sub_42A799`
/// (`sub_479B06(458, 1, 608, 49)` in transport coordinates, clipped to the transport: panel x
/// 474..607, y 1..48; docs/ui_spec.md section 3).
pub fn on_logo(x: f32, y: f32) -> bool {
    (474.0..608.0).contains(&x) && (1.0..49.0).contains(&y)
}

/// The origins of the drum machine sections (panel pixels), for pointer math.
pub const TR808_ORIGIN: (f32, f32) = (90.0, 280.0);
pub const TR909_ORIGIN: (f32, f32) = (90.0, 395.0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_has_every_control() {
        let c = controls();
        assert_eq!(c.len(), 348);
        assert!(c.iter().all(|c| c.w > 0.0 && c.h > 0.0));
        let play = c.iter().find(|c| c.id == "transport.play").unwrap();
        assert_eq!((play.x, play.y, play.w, play.h), (112.0, 21.0, 32.0, 25.0));
        assert_eq!(play.family(), "btn_play_");
        assert_eq!(play.name(), "play");
    }

    #[test]
    fn backgrounds_match_the_art_generator() {
        let parts = rb_ui_art::parts::all_parts();
        let l = rb_ui_art::layout::layout(&parts);
        let mut want: Vec<(String, f32, f32)> = l
            .backgrounds
            .into_iter()
            .filter(|(n, _, _)| !n.starts_with("focus_"))
            .collect();
        let mut got: Vec<(String, f32, f32)> = backgrounds()
            .into_iter()
            .map(|(n, x, y)| (n.to_string(), x, y))
            .collect();
        want.sort_by(|a, b| a.partial_cmp(b).unwrap());
        got.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(got, want);
    }

    #[test]
    fn rows() {
        assert_eq!(row_at(100.0, 60.0), Some(0));
        assert_eq!(row_at(100.0, 509.0), Some(3));
        assert_eq!(row_at(10.0, 60.0), None);
        assert_eq!(row_at(100.0, 20.0), None);
    }

    #[test]
    fn the_logo() {
        assert!(on_logo(474.0, 1.0) && on_logo(607.9, 48.9));
        assert!(!on_logo(473.9, 10.0) && !on_logo(608.0, 10.0));
        assert!(!on_logo(500.0, 0.9) && !on_logo(500.0, 49.0));
    }
}
