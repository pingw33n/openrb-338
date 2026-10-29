//! The 1.x import's message stream (`orb_engine::load::load_1x`) against `tools/rb1x.py`, its
//! Python twin, on synthetic songs: a pseudo-random song block and every mapped controller of
//! tracks 2..5, enough events to flush a track buffer mid-walk. The user's 129 real 1.x songs are
//! compared by the env-gated `crates/orb-cli/tests/thirdparty.rs`.

#[path = "../../../tests/common/python.rs"]
mod python;

use orb_engine::load;
use rbs::rb1x::{BLOCK_LEN, Block, Event, Kind, Song1x};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// xorshift32
struct Rng(u32);
impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
}

fn song(seed: u32) -> Vec<u8> {
    let mut r = Rng(seed);
    let mut b = Box::new([0u8; BLOCK_LEN]);
    for x in b.iter_mut() {
        *x = r.next() as u8;
    }
    b[5..9].copy_from_slice(b"TinY");
    b[10..18].fill(0);
    b[13] = 1; // loop start bar 1
    b[17] = 8; // 8 bars
    let ccs: [&[u8]; 4] = [
        &[7, 10, 12, 14, 15, 16, 17, 18, 19, 20, 21, 22],
        &[7, 10, 12, 14, 15, 16, 22],
        &[7, 10, 12, 14, 15, 24, 27, 30, 42],
        &[
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
        ],
    ];
    let mut tracks = vec![
        vec![Event {
            time: 0,
            kind: Kind::Meta {
                kind: 2,
                data: b"File version 3.2.0 (c)1997".to_vec(),
            },
        }],
        vec![Event {
            time: 0,
            kind: Kind::Sysex(Block(b).encode()),
        }],
    ];
    for (i, set) in ccs.iter().enumerate() {
        let n = if i == 0 { 3000 } else { 400 };
        let mut t = 0;
        let mut ev = Vec::new();
        for _ in 0..n {
            t += r.next() % 3 * 8;
            ev.push(Event {
                time: t,
                kind: Kind::Cc {
                    ctrl: set[r.next() as usize % set.len()],
                    value: (r.next() % 128) as u8,
                },
            });
        }
        tracks.push(ev);
    }
    Song1x {
        format: 1,
        division: 32,
        tracks,
        version: (3, 2),
    }
    .write()
}

#[test]
fn equals_rb1x_py() {
    let dir = std::env::temp_dir().join(format!("rb1x_messages_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for seed in [1u32, 0x1234_5678, 0xDEAD_BEEF] {
        let data = song(seed);
        let path = dir.join(format!("{seed}.rbs"));
        std::fs::write(&path, &data).unwrap();
        let ours = load::load_1x(&Song1x::parse(&data).unwrap())
            .unwrap()
            .messages(0);
        let out = python::python()
            .arg("-c")
            .arg(
                "import os, struct, sys\n\
                 sys.path.insert(0, os.path.join(sys.argv[1], 'tools'))\n\
                 import rb1x\n\
                 m = rb1x.read_song(sys.argv[2]).load_messages()\n\
                 sys.stdout.buffer.write(struct.pack('<I', len(m)) + \
                 b''.join(struct.pack('<I', len(x)) + x for x in m))\n",
            )
            .arg(root())
            .arg(&path)
            .output()
            .expect("cannot run the rb1x dump");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let b = out.stdout;
        let u = |p: usize| u32::from_le_bytes(b[p..p + 4].try_into().unwrap()) as usize;
        let mut theirs = Vec::new();
        let mut p = 4;
        for _ in 0..u(0) {
            theirs.push(b[p + 4..p + 4 + u(p)].to_vec());
            p += 4 + u(p);
        }
        assert_eq!(ours.len(), theirs.len(), "seed {seed}: message count");
        for (i, (a, t)) in ours.iter().zip(&theirs).enumerate() {
            assert!(a == t, "seed {seed}: message {i} differs");
        }
        // the 303 #1 buffer flushed mid-walk
        assert!(ours.iter().filter(|m| m[2] == 37 && m[3] == 1).count() >= 2);
    }
    let _ = std::fs::remove_dir_all(&dir);
}
