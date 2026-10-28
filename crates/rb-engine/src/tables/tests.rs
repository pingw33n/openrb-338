//! The generated tables against the original's run-time dumps (`tests/fixtures/tables`, from
//! rbhost `--dump-tables`), and the static tables and builder constants against the bytes of
//! ReBirth.dll (taken from the CD image, `RB_CD_ISO`, by `rb-testkit`; they skip without it),
//! read by file offset.

use super::data::*;
use super::*;
use std::cell::Cell;
use std::path::{Path, PathBuf};

thread_local! {
    /// The smallest distance, in f64 ulps, of an f32 store's value from an f32 rounding midpoint
    /// since the last [`take_margin`].
    static MARGIN: Cell<f64> = const { Cell::new(f64::INFINITY) };
}

/// Called by [`st`] for every f32 store of a generator.
pub(super) fn record_store(x: f64) {
    if x == 0.0 || !x.is_finite() {
        return;
    }
    let r = x as f32;
    let a = x.abs();
    let ra = r.abs();
    // the f32 neighbor of r on x's side, then the midpoint between them (exact in f64)
    let nb = if a > ra as f64 {
        f32::from_bits(ra.to_bits() + 1)
    } else {
        f32::from_bits(ra.to_bits() - 1)
    };
    let mid = (ra as f64 + nb as f64) / 2.0;
    let ulp = f64::from_bits(a.to_bits() + 1) - a;
    let m = (a - mid).abs() / ulp;
    MARGIN.with(|c| c.set(c.get().min(m)));
}

fn take_margin() -> f64 {
    MARGIN.with(|c| c.replace(f64::INFINITY))
}

/// The rounding margin of every generator's f32 stores: a math library that is off by fewer f64
/// ulps than this (before error growth through the formula) cannot change a stored f32.
#[test]
fn store_margins() {
    take_margin();
    let mut all = f64::INFINITY;
    let mut report = Vec::new();
    let builds: [(&str, fn()); 9] = [
        ("t", || {
            level_curve();
        }),
        ("pitch303", || {
            tb303_pitch();
        }),
        ("filt303", || {
            tb303_filter();
        }),
        ("pcf_lp", || {
            pcf_lowpass();
        }),
        ("pcf_bp", || {
            pcf_bandpass();
        }),
        ("dist_w", || {
            dist_shaper();
        }),
        ("dt909", || {
            tr909_decay();
        }),
        ("smooth303", || {
            tb303_smoother();
        }),
        ("smooth_pcf", || {
            pcf_smoother();
        }),
    ];
    for (name, build) in builds {
        build();
        let m = take_margin();
        report.push(format!("{name} {m}"));
        all = all.min(m);
    }
    eprintln!("store margins (f64 ulps): {}", report.join(", "));
    assert!(all >= 1358.0, "min margin {all}: {report:?}");
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn fixture(name: &str) -> Vec<u8> {
    let p = root().join("tests/fixtures/tables").join(name);
    std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn fixture_f32(name: &str) -> Vec<u32> {
    let b = fixture(name);
    let (chunks, rest) = b.as_chunks::<4>();
    assert!(rest.is_empty(), "{name}: length");
    chunks.iter().map(|&c| u32::from_le_bytes(c)).collect()
}

fn fixture_u16(name: &str) -> Vec<u16> {
    let b = fixture(name);
    let (chunks, rest) = b.as_chunks::<2>();
    assert!(rest.is_empty(), "{name}: length");
    chunks.iter().map(|&c| u16::from_le_bytes(c)).collect()
}

/// Asserts `got` equals the dump bit for bit; on a mismatch reports the count, the first entries
/// and the largest difference in ulps.
fn assert_f32(name: &str, got: &[f32]) {
    let want = fixture_f32(name);
    assert_eq!(got.len(), want.len(), "{name}: length");
    let bad: Vec<(usize, u32, u32)> = got
        .iter()
        .zip(&want)
        .enumerate()
        .filter(|(_, (g, w))| g.to_bits() != **w)
        .map(|(i, (g, w))| (i, g.to_bits(), *w))
        .collect();
    let max_ulps = bad
        .iter()
        .map(|&(_, g, w)| (g as i64 - w as i64).abs())
        .max()
        .unwrap_or(0);
    assert!(
        bad.is_empty(),
        "{name}: {} of {} entries differ, up to {max_ulps} ulps; first (index, got, dump): {:08X?}",
        bad.len(),
        got.len(),
        &bad[..bad.len().min(8)]
    );
}

#[test]
fn level_curve_matches_dump() {
    let t = level_curve();
    assert_f32("t.bin", &t);
    assert_eq!(t[127], 1.0);
    assert_eq!(t[0], 0.0);
}

#[test]
fn pan_matches_dump() {
    let pan = pan_table(&level_curve());
    assert_f32("pan.bin", pan.as_flattened().as_flattened());
    // the asymmetric law: b = 0 silences R, b = 127 leaves L = T[a - 124]
    let t = level_curve();
    assert_eq!(pan[127][0], [t[127], t[0]]);
    assert_eq!(pan[127][127], [t[127 - 124], t[127]]);
}

#[test]
fn tb303_pitch_matches_dump() {
    let p = tb303_pitch();
    assert_f32("pitch303.bin", &p[..]);
    // index 3072 is C2, 65.4 Hz (the phase runs from -1 to 1)
    assert!((p[3072] as f64 * 22050.0 - 65.406).abs() < 0.001);
}

#[test]
fn tb303_filter_matches_dump() {
    assert_f32("filt303.bin", tb303_filter().as_flattened().as_flattened());
}

#[test]
fn pcf_matches_dump() {
    assert_f32("pcf_lp.bin", pcf_lowpass().as_flattened().as_flattened());
    assert_f32("pcf_bp.bin", pcf_bandpass().as_flattened().as_flattened());
}

#[test]
fn dist_shaper_matches_dump() {
    let w = dist_shaper();
    assert_f32("dist_w.bin", w.as_flattened());
    assert_eq!(w[0][128].to_bits(), 0x8000_0000, "W[t][128] is -0.0");
}

#[test]
fn drum_tables_match_dump() {
    assert_eq!(&tr808_tom_step()[..], &fixture_u16("t808.bin")[..]);
    assert_eq!(&tr909_tom_step()[..], &fixture_u16("t909.bin")[..]);
    assert_f32("dt909.bin", &tr909_decay());
    assert_eq!(tr808_tom_step()[64], 256);
    assert_eq!(tr909_tom_step()[0], 188);
}

#[test]
fn smoothers_match_dump() {
    for (name, s) in [
        ("smooth303.bin", tb303_smoother()),
        ("smooth_pcf.bin", pcf_smoother()),
    ] {
        assert_f32(name, &[s.b2, s.a, s.g]);
    }
}

#[test]
fn tables_new_builds_everything() {
    let t = Tables::new();
    assert_eq!(t.pan[64][64], [t.level[64], t.level[64]]);
    assert_eq!(t.tb303_filter[7][1023], tb303_filter()[7][1023]);
}

// ------------------------------------------------------------------ the DLL's bytes

/// ReBirth.dll, addressed by virtual address through its section table.
struct Dll {
    bytes: Vec<u8>,
    /// (virtual address, virtual size, raw offset, raw size)
    sections: Vec<(u32, u32, u32, u32)>,
}

impl Dll {
    /// The DLL of the CD image (`RB_CD_ISO`, by `rb-testkit`; byte for byte `original/`'s), or
    /// `None` (a skip with the message) when it is not set.
    fn open() -> Option<Self> {
        let cd = rb_testkit::cd_files()?;
        let bytes = std::fs::read(&cd.dll).unwrap();
        let u16_at = |o: usize| u16::from_le_bytes(bytes[o..o + 2].try_into().unwrap());
        let u32_at = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
        let pe = u32_at(0x3C) as usize;
        assert_eq!(&bytes[pe..pe + 4], b"PE\0\0");
        let count = u16_at(pe + 6) as usize;
        let opt = u16_at(pe + 20) as usize;
        let base = u32_at(pe + 24 + 28);
        assert_eq!(base, 0x1000_0000);
        let sections = (0..count)
            .map(|i| {
                let s = pe + 24 + opt + 40 * i;
                (
                    base + u32_at(s + 12),
                    u32_at(s + 8),
                    u32_at(s + 20),
                    u32_at(s + 16),
                )
            })
            .collect();
        Some(Dll { bytes, sections })
    }

    fn read(&self, va: u32, len: usize) -> &[u8] {
        for &(sva, vsize, raw, rsize) in &self.sections {
            if va >= sva && va - sva < vsize.max(rsize) {
                let off = va - sva;
                assert!(
                    off as usize + len <= rsize as usize,
                    "{va:#X}: not in the file (zero-filled)"
                );
                let o = (raw + off) as usize;
                return &self.bytes[o..o + len];
            }
        }
        panic!("{va:#X}: no section");
    }

    fn u32(&self, va: u32) -> u32 {
        u32::from_le_bytes(self.read(va, 4).try_into().unwrap())
    }

    fn f64(&self, va: u32) -> f64 {
        f64::from_le_bytes(self.read(va, 8).try_into().unwrap())
    }

    fn f32(&self, va: u32) -> f32 {
        f32::from_bits(self.u32(va))
    }

    /// The imm32 of `mov r32, imm32` (opcode B8+r) at `va`.
    fn mov_imm(&self, va: u32) -> u32 {
        let op = self.read(va, 1)[0];
        assert!((0xB8..=0xBF).contains(&op), "{va:#X}: not mov r32, imm32");
        self.u32(va + 1)
    }
}

fn le_bytes<T: Copy, const W: usize>(v: &[T], f: impl Fn(T) -> [u8; W]) -> Vec<u8> {
    v.iter().flat_map(|&x| f(x)).collect()
}

#[test]
fn static_tables_match_dll() {
    let Some(dll) = Dll::open() else {
        return;
    };
    let f32s = |v: &[f32]| le_bytes(v, f32::to_le_bytes);
    let u16s = |v: &[u16]| le_bytes(v, u16::to_le_bytes);
    let i16s = |v: &[i16]| le_bytes(v, i16::to_le_bytes);
    let tables: [(&str, u32, Vec<u8>); 17] = [
        ("LEVEL_STEP", 0x1014_F8E4, f32s(&[LEVEL_STEP])),
        ("TB303_TUNE", 0x1003_3F90, i16s(&TB303_TUNE)),
        ("DIST_GRID", 0x1015_1AB8, f32s(DIST_GRID.as_flattened())),
        ("PCF_DECAY", 0x1003_32D0, f32s(&PCF_DECAY)),
        ("PCF_LENGTHS", 0x1003_3310, u16s(&PCF_LENGTHS)),
        ("PCF_RESOLUTIONS", 0x1003_3380, u16s(&PCF_RESOLUTIONS)),
        ("PCF_STEPS", 0x1003_33F0, PCF_STEPS.as_flattened().to_vec()),
        ("PCF_DATA", 0x1003_3310, PCF_DATA.to_vec()),
        ("TR808_SD_SAMPLE", 0x1003_4090, i16s(&TR808_SD_SAMPLE)),
        ("TR808_SD_NOISE_SEQ", 0x1003_409C, u16s(&TR808_SD_NOISE_SEQ)),
        (
            "TR808_SD_NOISE_SAMPLE",
            0x1003_40DC,
            u16s(&TR808_SD_NOISE_SAMPLE),
        ),
        ("TR808_TONE", 0x1003_40E4, f32s(&TR808_TONE)),
        ("TR808_DECAY", 0x1003_4104, f32s(&TR808_DECAY)),
        ("TR909_SD_SAMPLE", 0x1003_4210, i16s(&TR909_SD_SAMPLE)),
        ("TR909_SD_NOISE_SEQ", 0x1003_421C, u16s(&TR909_SD_NOISE_SEQ)),
        (
            "TR909_SD_NOISE_SAMPLE",
            0x1003_425C,
            u16s(&TR909_SD_NOISE_SAMPLE),
        ),
        ("TR909_BD_ATTACK", 0x1003_4264, f32s(&TR909_BD_ATTACK)),
    ];
    for (name, va, want) in tables {
        assert_eq!(dll.read(va, want.len()), &want[..], "{name} at {va:#X}");
    }
    // the noise sequences only index the four-entry sample maps (909: three used)
    assert!(TR808_SD_NOISE_SEQ.iter().all(|&s| s < 4));
    assert!(TR909_SD_NOISE_SEQ.iter().all(|&s| s < 3));
    // PCF_DATA ends where `.data`'s file part ends (raw size 0x4400 at 0x10030000; above it
    // is BSS, computed at run time)
    assert_eq!(0x1003_3310 + PCF_DATA.len(), 0x1003_4400);
}

#[test]
fn builder_constants_match_dll() {
    let Some(dll) = Dll::open() else {
        return;
    };
    let f64s: &[(u32, f64)] = &[
        // TB-303 pitch, sub_10023920
        (0x1015_3A90, C_10153A90),
        (0x1015_3A98, C_10153A98),
        (0x1015_3A80, C_10153A80),
        (0x1015_3AB0, C_10153AB0),
        // TB-303 FILT, sub_100234A0
        (0x1015_39C8, C_101539C8),
        (0x1015_39D8, C_101539D8),
        (0x1015_39E0, C_101539E0),
        (0x1015_39E8, C_101539E8),
        (0x1015_39F0, C_101539F0),
        (0x1015_39F8, C_101539F8),
        (0x1015_3A00, C_10153A00),
        (0x1015_3A08, C_10153A08),
        (0x1015_3A10, C_10153A10),
        (0x1015_3A18, C_10153A18),
        (0x1015_3A20, C_10153A20),
        (0x1015_39A8, C_101539A8),
        (0x1015_3A28, C_10153A28),
        (0x1015_3A30, C_10153A30),
        (0x1015_3A38, C_10153A38),
        (0x1015_3A40, C_10153A40),
        (0x1015_3A48, C_10153A48),
        (0x1015_3A50, C_10153A50),
        // smoothers, sub_10026BB0 and sub_10017840
        (0x1015_3DE0, SMOOTH_W),
        (0x1015_3DE8, SMOOTH_HALF_NEG_W),
        (0x1015_3DF0, SMOOTH_TWO),
        (0x1015_3DF8, SMOOTH_NEG_W),
        (0x1015_3E00, SMOOTH_ONE),
        (0x1015_2490, SMOOTH_W),
        (0x1015_2498, SMOOTH_HALF_NEG_W),
        (0x1015_24A0, SMOOTH_TWO),
        (0x1015_24A8, SMOOTH_NEG_W),
        (0x1015_24B0, SMOOTH_ONE),
        // PCF band-pass extra
        (0x1015_2468, C_10152468),
        // distortion, sub_10011210
        (0x1015_1BE0, C_10151BE0),
        (0x1015_1BF0, C_10151BF0),
        (0x1015_1BE8, C_10151BE8),
        // drums
        (0x1015_3EA0, C_10153EA0),
        (0x1015_3EA8, C_10153EA8),
        (0x1015_42A0, C_101542A0),
        (0x1015_42B0, C_101542B0),
        (0x1015_42A8, C_101542A8),
    ];
    let f32s: &[(u32, f32)] = &[
        (0x1015_3A58, C_10153A58),
        (0x1015_3A5C, C_10153A5C),
        (0x1015_3A60, C_10153A60),
        (0x1015_3A64, C_10153A64),
        (0x1015_3A68, C_10153A68),
        (0x1015_23CC, C_101523CC),
        (0x1015_1BFC, C_10151BFC),
        (0x1015_1C00, C_10151C00),
        (0x1015_1C04, C_10151C04),
        (0x1015_1C08, C_10151C08),
        (0x1015_1C0C, C_10151C0C),
        (0x1015_431C, C_1015431C),
    ];
    for &(va, v) in f64s {
        assert_eq!(dll.f64(va).to_bits(), v.to_bits(), "f64 at {va:#X}");
    }
    for &(va, v) in f32s {
        assert_eq!(dll.f32(va).to_bits(), v.to_bits(), "f32 at {va:#X}");
    }
    // the two PCF builders' data, in the order of PcfConsts
    for (c, f64_vas, f32_vas) in [
        (
            &PCF_LP,
            [
                0x1015_2340,
                0x1015_2350,
                0x1015_2358,
                0x1015_2360,
                0x1015_2368,
                0x1015_2370,
                0x1015_2378,
                0x1015_2380,
                0x1015_2388,
                0x1015_2390,
                0x1015_2398,
                0x1015_2328,
                0x1015_23A0,
                0x1015_23A8,
                0x1015_23B0,
                0x1015_23B8,
            ],
            [
                0x1015_23C0,
                0x1015_23C4,
                0x1015_23C8,
                0x1015_23D0,
                0x1015_23D4,
            ],
        ),
        (
            &PCF_BP,
            [
                0x1015_23F0,
                0x1015_2400,
                0x1015_2408,
                0x1015_2410,
                0x1015_2418,
                0x1015_2420,
                0x1015_2428,
                0x1015_2430,
                0x1015_2438,
                0x1015_2440,
                0x1015_2448,
                0x1015_23D8,
                0x1015_2450,
                0x1015_2458,
                0x1015_2460,
                0x1015_2470,
            ],
            [
                0x1015_2478,
                0x1015_247C,
                0x1015_2480,
                0x1015_2488,
                0x1015_2484,
            ],
        ),
    ] {
        let f64_vals = [
            c.c04,
            c.c052,
            c.c16,
            c.c05,
            c.c06,
            c.c128,
            c.c512,
            c.cq,
            c.c03,
            c.one,
            c.two_pi,
            c.one_d,
            c.neg_pi,
            c.two,
            c.neg_two_pi,
            c.c08,
        ];
        let f32_vals = [c.c17, c.one_f, c.zero_f, c.four_f, c.three_f];
        for (va, v) in f64_vas.into_iter().zip(f64_vals) {
            assert_eq!(dll.f64(va).to_bits(), v.to_bits(), "PCF f64 at {va:#X}");
        }
        for (va, v) in f32_vas.into_iter().zip(f32_vals) {
            assert_eq!(dll.f32(va).to_bits(), v.to_bits(), "PCF f32 at {va:#X}");
        }
    }
    let hi = |v: f64| (v.to_bits() >> 32) as u32;
    let lo = |v: f64| v.to_bits() as u32;
    let imms: &[(u32, u32)] = &[
        (0x1000_46B4, T_TOP.to_bits()),
        // pitch: pow(2.0, 1/12), the cube, pow(2.0, 1/768), 3072.0
        (0x1002_3A8E, hi(2.0)),
        (0x1002_3A93, lo(PITCH_SEMI_EXP)),
        (0x1002_3A9B, hi(PITCH_SEMI_EXP)),
        (0x1002_3AC3, 3),
        (0x1002_3AB9, hi(2.0)),
        (0x1002_3AF0, lo(PITCH_STEP_EXP)),
        (0x1002_3B02, hi(PITCH_STEP_EXP)),
        (0x1002_3B0E, hi(PITCH_ORIGIN)),
        (0x1002_3B20, hi(PITCH_ORIGIN)),
        // FILT: pow(1602.0, 1/1023), 1023.0, 1.78f
        (0x1002_34B7, lo(FILT_SPAN)),
        (0x1002_34BC, hi(FILT_SPAN)),
        (0x1002_34C1, lo(FILT_SPAN_EXP)),
        (0x1002_34C9, hi(FILT_SPAN_EXP)),
        (0x1002_34DA, hi(FILT_TOP)),
        (0x1002_34E6, hi(FILT_TOP)),
        (0x1002_3507, lo(Q_BASE)),
        (0x1002_3513, hi(Q_BASE)),
        // PCF LP and BP: K, 511.0, 1.78f, K in the loop
        (0x1001_72CE, lo(PCF_K)),
        (0x1001_72D3, hi(PCF_K)),
        (0x1001_72DD, hi(PCF_TOP)),
        (0x1001_72FB, lo(Q_BASE)),
        (0x1001_7307, hi(Q_BASE)),
        (0x1001_73A1, lo(PCF_K)),
        (0x1001_73AA, hi(PCF_K)),
        (0x1001_757E, lo(PCF_K)),
        (0x1001_7583, hi(PCF_K)),
        (0x1001_758D, hi(PCF_TOP)),
        (0x1001_75AB, lo(Q_BASE)),
        (0x1001_75B7, hi(Q_BASE)),
        (0x1001_7651, lo(PCF_K)),
        (0x1001_765A, hi(PCF_K)),
        // 808 and 909: pow(2.0, 1/192 or 1/144), the 64th power
        (0x1002_7066, hi(2.0)),
        (0x1002_706B, lo(T808_EXP)),
        (0x1002_7073, hi(T808_EXP)),
        (0x1002_7093, 64),
        (0x1002_7089, hi(2.0)),
        (0x1002_A476, hi(2.0)),
        (0x1002_A47B, lo(T909_EXP)),
        (0x1002_A483, hi(T909_EXP)),
        (0x1002_A4A3, 64),
        (0x1002_A499, hi(2.0)),
    ];
    for &(va, v) in imms {
        assert_eq!(dll.mov_imm(va), v, "immediate at {va:#X}");
    }
}
