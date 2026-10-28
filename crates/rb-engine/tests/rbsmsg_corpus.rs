//! The load message stream against `tools/rbhost/rbsmsg.py`, the validated reference (rbhost
//! renders from its messages equal the app's exports byte for byte, `docs/export_state.md`
//! section 13), for every song of `tests/songs/shipped` and `tests/songs/diag/songs`.
//!
//! For a song with the built-in mod the whole `.rbhm` file of `rbsmsg.message_file` is compared;
//! for a mod song (rbsmsg refuses those, mods are m-2) the message list of
//! `rbsmsg.load_messages`.

#[path = "../../../tests/common/python.rs"]
mod python;

use rb_engine::load;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn songs() -> Vec<PathBuf> {
    let mut v = Vec::new();
    for dir in ["tests/songs/shipped", "tests/songs/diag/songs"] {
        for e in std::fs::read_dir(root().join(dir)).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_some_and(|x| x == "rbs") {
                v.push(p);
            }
        }
    }
    v.sort();
    v
}

/// Per song: a kind byte (1 = an .rbhm file, 0 = a message list), a u32 length, the blob.
const DUMP: &str = r#"
import os, struct, sys
sys.path.insert(0, os.path.join(sys.argv[1], "tools", "rbhost"))
import rbsmsg, rbs
out = sys.stdout.buffer
for p in sys.argv[2:]:
    song = rbs.read_song(p)
    if song.glob.mod_name == rbsmsg.STANDARD_MOD:
        kind, blob = 1, rbsmsg.message_file(song)
    else:
        msgs = rbsmsg.load_messages(song)
        kind = 0
        blob = struct.pack("<I", len(msgs)) + b"".join(struct.pack("<I", len(m)) + m for m in msgs)
    out.write(bytes([kind]) + struct.pack("<I", len(blob)) + blob)
"#;

fn u32le(b: &[u8], p: usize) -> u32 {
    u32::from_le_bytes(b[p..p + 4].try_into().unwrap())
}

fn framed(msgs: &[Vec<u8>]) -> Vec<u8> {
    let mut b = (msgs.len() as u32).to_le_bytes().to_vec();
    for m in msgs {
        b.extend((m.len() as u32).to_le_bytes());
        b.extend(m);
    }
    b
}

/// The .rbhm file of tools/rbhost/rbsmsg.py (its docstring has the format).
fn rbhm(song: &rb_engine::song::Song, msgs: &[Vec<u8>]) -> Vec<u8> {
    let g = &song.globals;
    let mut b = b"RBHM".to_vec();
    b.extend(1u32.to_le_bytes());
    b.extend(411u16.to_le_bytes()); // "Standard ReBirth", exe sub_417AE9
    for v in [g.tempo, g.loop_start, g.loop_end] {
        b.extend(v.to_le_bytes());
    }
    b.extend(framed(msgs));
    b
}

#[test]
fn equals_rbsmsg() {
    let songs = songs();
    assert!(songs.len() >= 163, "found only {} songs", songs.len());
    let out = python::python()
        .arg("-c")
        .arg(DUMP)
        .arg(root())
        .args(&songs)
        .output()
        .expect("run the rbsmsg dump");
    assert!(
        out.status.success(),
        "rbsmsg.py failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let dump = out.stdout;
    let (mut p, mut rbhm_songs, mut mod_songs, mut messages, mut events) = (0, 0, 0, 0, 0);
    for path in &songs {
        let kind = dump[p];
        let len = u32le(&dump, p + 1) as usize;
        let want = &dump[p + 5..p + 5 + len];
        p += 5 + len;
        let rbs_song = rbs::Song::parse(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(rbs_song.head.revision, 0, "{}", path.display());
        let loaded = load::load(&rbs_song).unwrap();
        let msgs = loaded.messages(0);
        let got = if kind == 1 {
            rbhm_songs += 1;
            rbhm(&loaded.song, &msgs)
        } else {
            mod_songs += 1;
            framed(&msgs)
        };
        if got != want {
            let at = got.iter().zip(want).position(|(a, b)| a != b);
            panic!(
                "{}: differs from rbsmsg.py at byte {at:?} (lengths {} vs {})",
                path.display(),
                got.len(),
                want.len()
            );
        }
        messages += msgs.len();
        events += loaded.song.tracks.iter().map(Vec::len).sum::<usize>();
    }
    assert_eq!(p, dump.len());
    assert_eq!(mod_songs, 8);
    eprintln!(
        "{} songs equal ({rbhm_songs} .rbhm files, {mod_songs} mod songs' message lists); \
         {messages} messages, {events} track events",
        songs.len()
    );
}
