//! `recrypt`: rewrites a mod so that the key bytes orb embeds are all it needs (task-059,
//! docs/rbm.md section 12).
//!
//! The key pages depend only on the sums over the first 1024 bytes of the EMBF payloads, which
//! are never scrambled, and a file's table bytes depend only on its reader's key, i.e. its chunk
//! ordinal. So the rewrite: decrypts every file with the mod's real table; adds a 1024-byte EMBF
//! (never scrambled) whose two words steer both sums to [`embedded::WINDOW`]; puts every
//! scrambled file at an ordinal whose period, under its reader's key, is at most
//! [`embedded::WINDOW_LEN`] (with zero-size filler chunks between where needed; the 2047-byte
//! window fits every key, so none is); and scrambles each file again against the embedded
//! window under its new key.
//!
//! Zero-size chunks of the filler id `"    "` in the input are dropped (they are taken for
//! fillers of an earlier recrypt; nothing reads such a chunk), as is an earlier pad.

use std::collections::HashMap;

use crate::{
    CLEAR, Chunk, DAT_HEADER, KeyTable, PAGE, Rbm, RbmError, Reader, SUM_INIT, be32, embedded,
    period, scramble,
};

/// The EMBF that steers the key sums: its name (outside the 46 slot names and the exe's
/// `%d.jpg`, `Splash.jpg` and demo-song names, so nothing reads it).
pub const PAD_NAME: &str = "rb-recrypt.pad";

/// The most EMBF chunks the original takes: `COpenMod` (`sub_41813E`) has 200 entries of 276
/// bytes at +1692 (0x4182ED) and writes entry `count` with no bound check (0x418772).
pub const MAX_EMBF: usize = 200;

/// The filler chunk: the IFF 85 filler id, four spaces, with no payload. The original's chunk
/// walk counts it (`sub_41813E` 0x418832) and nothing reads it.
const FILLER: [u8; 4] = *b"    ";

/// The pad's payload size: at most `CLEAR`, so it is never scrambled.
const PAD_LEN: usize = CLEAR;

struct File {
    name_lower: String,
    payload: Vec<u8>,
    reader: Reader,
    /// The previous file of the same name (case-insensitive): it must stay first.
    prev: Option<usize>,
}

impl File {
    fn scrambled(&self) -> bool {
        self.payload.len() > CLEAR
    }
}

enum Item {
    Chunk([u8; 4], Vec<u8>),
    File(usize),
    Pad,
    Filler,
}

/// The two key sums of a set of EMBF payloads (docs/rbm.md 5.2), without the initial values.
fn sums_of<'a>(payloads: impl Iterator<Item = &'a [u8]>) -> [u32; 2] {
    let mut acc = [0u32; 2];
    for p in payloads {
        let n = p.len().min(CLEAR);
        let mut buf = [0x65u8; CLEAR];
        buf[..n].copy_from_slice(&p[..n]);
        for (j, a) in acc.iter_mut().enumerate() {
            let s = (0..128).fold(0u32, |s, w| s.wrapping_add(be32(&buf, j * 512 + w * 4)));
            *a = a.wrapping_add(s);
        }
    }
    acc
}

fn push_chunk(out: &mut Vec<u8>, tag: &[u8; 4], payload: &[u8]) -> Result<(), RbmError> {
    let size =
        u32::try_from(payload.len()).map_err(|_| RbmError::Recrypt("a chunk over 4 GiB".into()))?;
    out.extend_from_slice(tag);
    out.extend_from_slice(&size.to_be_bytes());
    out.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        out.push(0);
    }
    Ok(())
}

/// Rewrites `rbm` so that it loads with [`KeyTable::embedded`] alone, in orb and in the original.
/// `table` is the mod's own table (from `RB20FUL.DAT`, or the embedded one for a mod that is
/// already recrypted). Every embedded file keeps its name, bytes and lookup order; the non-EMBF
/// chunks keep their bytes (those before the first EMBF stay first, the others go last); a pad
/// EMBF ([`PAD_NAME`]) and zero-size filler chunks are added (those of an earlier recrypt are
/// dropped).
pub fn recrypt(rbm: &Rbm, table: &KeyTable) -> Result<Vec<u8>, RbmError> {
    recrypt_with(rbm, table, &[])
}

/// [`recrypt`], where the embedded files at the chunk ordinals `stored_plain` are taken as stored
/// (a zero key: the mod's maker scrambled them with a key half of zeros, docs/rbm.md 12.5)
/// instead of descrambled with `table`.
pub fn recrypt_with(
    rbm: &Rbm,
    table: &KeyTable,
    stored_plain: &[usize],
) -> Result<Vec<u8>, RbmError> {
    let is_old_pad =
        |c: &Chunk| &c.tag == b"EMBF" && c.name.as_deref() == Some(PAD_NAME) && c.size == PAD_LEN;
    let is_filler = |c: &Chunk| c.tag == FILLER && c.size == 0;
    let first_embf = rbm
        .chunks()
        .iter()
        .position(|c| &c.tag == b"EMBF")
        .unwrap_or(rbm.chunks().len());

    let mut lead = Vec::new();
    let mut tail = Vec::new();
    let mut files: Vec<File> = Vec::new();
    let mut last_of: HashMap<String, usize> = HashMap::new();
    for c in rbm.chunks() {
        if is_filler(c) || is_old_pad(c) {
            continue;
        }
        if &c.tag == b"EMBF" {
            let name_lower = c.name.as_deref().unwrap_or("").to_ascii_lowercase();
            let prev = last_of.insert(name_lower.clone(), files.len());
            files.push(File {
                name_lower,
                payload: if stored_plain.contains(&c.ordinal) {
                    rbm.raw_payload(c).to_vec()
                } else {
                    rbm.payload(c, table)?
                },
                reader: rbm.reader(c),
                prev,
            });
        } else {
            let item = (c.tag, rbm.raw_payload(c).to_vec());
            if c.ordinal < first_embf {
                lead.push(item);
            } else {
                tail.push(item);
            }
        }
    }
    if files
        .iter()
        .any(|f| f.name_lower == PAD_NAME.to_ascii_lowercase())
    {
        return Err(RbmError::Recrypt(format!(
            "the mod already has a file named {PAD_NAME}"
        )));
    }
    if files.len() + 1 > MAX_EMBF {
        return Err(RbmError::Recrypt(format!(
            "{} embedded files and the pad are more than the original's {MAX_EMBF}",
            files.len()
        )));
    }

    // The layout: ordinal by ordinal, the first file (in file order, after its earlier
    // namesakes) whose reader's period at this ordinal fits the window, else a file that is not
    // scrambled, else a filler.
    let len = embedded::WINDOW_LEN;
    let mut items: Vec<Item> = lead.into_iter().map(|(t, p)| Item::Chunk(t, p)).collect();
    items.push(Item::Pad);
    let mut placed = vec![false; files.len()];
    let mut left = files.iter().filter(|f| f.scrambled()).count();
    let free = |i: usize, placed: &[bool]| !placed[i] && files[i].prev.is_none_or(|p| placed[p]);
    while left > 0 {
        let o = items.len();
        let fits = |r: Reader| period(r.key(o)) <= len;
        // In file order: with a window that fits every key (2047 bytes) this keeps the
        // original order and needs no filler.
        let pick = (0..files.len())
            .find(|&i| free(i, &placed) && (!files[i].scrambled() || fits(files[i].reader)));
        match pick {
            Some(i) => {
                placed[i] = true;
                if files[i].scrambled() {
                    left -= 1;
                }
                items.push(Item::File(i));
            }
            None => items.push(Item::Filler),
        }
    }
    for (i, p) in placed.iter_mut().enumerate() {
        if !*p {
            *p = true;
            items.push(Item::File(i));
        }
    }
    items.extend(tail.into_iter().map(|(t, p)| Item::Chunk(t, p)));

    // The pad's two words make both sums the window's offset in the key material.
    let target = u32::try_from(embedded::WINDOW - DAT_HEADER).expect("the window is in the DAT");
    let mut pad = vec![0u8; PAD_LEN];
    pad[..PAD_NAME.len()].copy_from_slice(PAD_NAME.as_bytes());
    let s = sums_of(
        files
            .iter()
            .map(|f| f.payload.as_slice())
            .chain(std::iter::once(pad.as_slice())),
    );
    for (j, at) in [(0usize, 508usize), (1, 1020)] {
        let have = SUM_INIT[j].wrapping_add(s[j]);
        pad[at..at + 4].copy_from_slice(&target.wrapping_sub(have).to_be_bytes());
    }

    let window = KeyTable::window();
    let mut out = Vec::with_capacity(rbm.data.len() + 8 * items.len() + PAD_LEN);
    out.extend_from_slice(b"CAT \0\0\0\0");
    out.extend_from_slice(&rbm.data[8..12]);
    for (o, item) in items.iter().enumerate() {
        match item {
            Item::Chunk(tag, p) => push_chunk(&mut out, tag, p)?,
            Item::Pad => push_chunk(&mut out, b"EMBF", &pad)?,
            Item::Filler => push_chunk(&mut out, &FILLER, &[])?,
            Item::File(i) => {
                let f = &files[*i];
                let mut p = f.payload.clone();
                scramble(&mut p, f.reader.key(o), &window)?;
                push_chunk(&mut out, b"EMBF", &p)?;
            }
        }
    }
    let size = u32::try_from(out.len() - 8)
        .map_err(|_| RbmError::Recrypt("the mod would be over 4 GiB".into()))?;
    out[4..8].copy_from_slice(&size.to_be_bytes());

    // What the original will compute from it.
    let back = Rbm::parse(out)?;
    let ok = back.key_pages() == [embedded::WINDOW; 2]
        && KeyTable::embedded(&back).is_some_and(|t| t.known() == [len; 2] && back.all_covered(&t))
        && len <= PAGE;
    if !ok {
        return Err(RbmError::Recrypt(
            "internal: the rewrite does not select the window".into(),
        ));
    }
    Ok(back.data)
}
