//! ReBirth RB-338 2.01 mods (`.rbm`): the container, the key table, the descrambling and the
//! mapping of the embedded AIFF files onto the engine's 46 sample slots (docs/rbm.md; the
//! reference implementation is `tools/rbm.py`).
//!
//! A mod is an IFF `CAT ` of type `PRBM`: `HEAD`, `EMBF` chunks (a NUL-terminated name, then one
//! file) and `INFO`. Every EMBF payload is stored in the clear up to byte 1024 and scrambled from
//! there on with a 4096-byte key table: two 2048-byte pages of the user's `RB20FUL.DAT`, at
//! offsets fixed by the mod's own bytes (task-053: the key material is read at run time from the
//! user's file; nothing of it is embedded or committed). Only those two pages are read.
//!
//! ```no_run
//! # use rbm::*; use rb_engine::samples::SampleSet;
//! # fn f(mod_bytes: Vec<u8>, builtin: &SampleSet) -> Result<(), RbmError> {
//! let rbm = Rbm::parse(mod_bytes)?;
//! let mut dat = std::fs::File::open("RB20FUL.DAT").map_err(RbmError::Io)?;
//! let table = KeyTable::read(&rbm, &mut dat)?;
//! let loaded = rbm.load_samples(&table, builtin)?;
//! # let _ = loaded; Ok(()) }
//! ```
//!
//! Nothing here panics on malformed input; every defect is an [`RbmError`].

use std::fmt;
use std::io::{self, Read, Seek, SeekFrom};
use std::sync::Arc;

use rb_engine::samples::{self, Sample, SampleSet};

/// EMBF payload bytes `[0, CLEAR)` are stored as they are; the rest is scrambled.
pub const CLEAR: usize = 1024;
/// `RB20FUL.DAT`: a 256-byte header, then 2^27 bytes of key material.
pub const DAT_HEADER: u64 = 256;
/// The size of the key material of `RB20FUL.DAT`.
pub const DAT_BODY: u64 = 0x800_0000;
/// The size of one key page.
pub const PAGE: usize = 2048;
/// The size of the key table: two pages.
pub const TABLE_LEN: usize = 2 * PAGE;
/// The magic at the start of `RB20FUL.DAT` (big-endian).
pub const DAT_MAGIC: u32 = 0x03E1_F11E;
/// The initial key sums.
const SUM_INIT: [u32; 2] = [0x2000, 0x3000];

/// The 808 sample file names, slots 1..=24 (resource IDs 1000..=1023).
pub const SLOTS_808: [&str; 24] = [
    "TR808BD.aif",
    "TR808SD1.aif",
    "TR808SD2.aif",
    "TR808SD3.aif",
    "TR808SD4.aif",
    "TR808SD5.aif",
    "TR808N1.aif",
    "TR808N2.aif",
    "TR808N3.aif",
    "TR808N4.aif",
    "TR808CB.aif",
    "TR808CH.aif",
    "TR808CL.aif",
    "TR808CP.aif",
    "TR808CY.aif",
    "TR808HC.aif",
    "TR808HT.aif",
    "TR808LC.aif",
    "TR808LT.aif",
    "TR808MA.aif",
    "TR808MC.aif",
    "TR808MT.aif",
    "TR808OH.aif",
    "TR808RS.aif",
];
/// The 909 sample file names, slots 25..=45 (resource IDs 1025..=1045).
pub const SLOTS_909: [&str; 21] = [
    "TR909bd1.aif",
    "TR909bd2.aif",
    "TR909bd3.aif",
    "TR909bd4.aif",
    "TR909sd1.aif",
    "TR909sd2.aif",
    "TR909sd3.aif",
    "TR909sd4.aif",
    "TR909sd5.aif",
    "TR909sdn1.aif",
    "TR909sdn2.aif",
    "TR909sdn3.aif",
    "TR909lt.aif",
    "TR909mt.aif",
    "TR909ht.aif",
    "TR909rs.aif",
    "TR909hc.aif",
    "TR909chh.aif",
    "TR909ohh.aif",
    "TR909ccy.aif",
    "TR909rcy.aif",
];
/// The startup sound's file name (resource ID 1024).
pub const STARTUP: &str = "Startup.aif";

/// The file name the mod uses for the sample with resource ID `id` (1000..=1045).
pub fn slot_name(id: u16) -> Option<&'static str> {
    let i = id.checked_sub(samples::FIRST_ID)? as usize;
    if i < 24 {
        Some(SLOTS_808[i])
    } else if id == samples::STARTUP_ID {
        Some(STARTUP)
    } else if id > samples::STARTUP_ID && id <= samples::LAST_ID {
        Some(SLOTS_909[(id - samples::STARTUP_ID - 1) as usize])
    } else {
        None
    }
}

/// What went wrong reading a mod.
#[derive(Debug)]
pub enum RbmError {
    /// The mod is not a valid IFF `CAT ` (the text says why).
    Format(String),
    /// The key file is not `RB20FUL.DAT` (bad magic) or is too short for the pages.
    KeyFile(String),
    /// Reading the key file failed.
    Io(io::Error),
    /// The key table is not 4096 bytes.
    KeyTable,
    /// An embedded file is not a usable AIFF (file name, reason).
    Sample(String, String),
    /// No embedded file has this name.
    NoSuchFile(String),
}

impl fmt::Display for RbmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RbmError::Format(m) => write!(f, "not a ReBirth mod: {m}"),
            RbmError::KeyFile(m) => write!(f, "RB20FUL.DAT: {m}"),
            RbmError::Io(e) => write!(f, "reading RB20FUL.DAT: {e}"),
            RbmError::KeyTable => write!(f, "the key table is not 4096 bytes"),
            RbmError::Sample(n, m) => write!(f, "the mod's sample {n}: {m}"),
            RbmError::NoSuchFile(n) => write!(f, "the mod has no file {n}"),
        }
    }
}

impl std::error::Error for RbmError {}

fn fmt_err(m: impl Into<String>) -> RbmError {
    RbmError::Format(m.into())
}

/// One chunk of the CAT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// The index among all chunks of the CAT, `HEAD` = 0.
    pub ordinal: usize,
    pub tag: [u8; 4],
    /// The absolute file offset of the payload.
    pub offset: usize,
    pub size: usize,
    /// EMBF only: the embedded file's name.
    pub name: Option<String>,
    /// EMBF only: `strlen(name) + 1`, where the file starts in the payload.
    pub name_len: usize,
}

impl Chunk {
    /// The descrambling key: the low byte of the ordinal.
    pub fn key(&self) -> u8 {
        self.ordinal as u8
    }
}

/// The `INFO` chunk (docs/rbm.md section 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Info {
    /// The description shown for the mod.
    pub text: String,
    pub ftp: String,
    pub web: String,
    /// The name in the Mods menu.
    pub name: String,
    /// `(major, minor, revision)`; `None` when the mod says "the app's version".
    pub version: Option<(u8, u8, u8)>,
}

/// A parsed mod. The file bytes are kept; embedded files are descrambled on request.
#[derive(Debug, Clone)]
pub struct Rbm {
    data: Vec<u8>,
    chunks: Vec<Chunk>,
    info: Option<Info>,
    warnings: Vec<String>,
}

fn cstr_latin1(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    b[..end].iter().map(|&c| c as char).collect()
}

fn be32(b: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

impl Rbm {
    /// Parses the container. A CAT type other than `PRBM` is accepted, as the original does,
    /// with a warning in [`Rbm::warnings`].
    pub fn parse(data: Vec<u8>) -> Result<Rbm, RbmError> {
        if data.len() < 12 || &data[..4] != b"CAT " {
            return Err(fmt_err("not an IFF CAT"));
        }
        let size = be32(&data, 4) as usize;
        if size.checked_add(8) != Some(data.len()) {
            return Err(fmt_err(format!(
                "CAT size {size} does not match the file ({} bytes)",
                data.len()
            )));
        }
        let mut warnings = Vec::new();
        if &data[8..12] != b"PRBM" {
            warnings.push(format!(
                "CAT type {:?} is not PRBM",
                String::from_utf8_lossy(&data[8..12])
            ));
        }
        let mut chunks = Vec::new();
        let mut info = None;
        let (mut pos, end) = (12usize, data.len());
        while pos + 8 <= end {
            let tag = [data[pos], data[pos + 1], data[pos + 2], data[pos + 3]];
            let csize = be32(&data, pos + 4) as usize;
            let body = pos + 8;
            let body_end = body
                .checked_add(csize)
                .filter(|&e| e <= end)
                .ok_or_else(|| fmt_err(format!("chunk at {pos} overruns the file")))?;
            let mut c = Chunk {
                ordinal: chunks.len(),
                tag,
                offset: body,
                size: csize,
                name: None,
                name_len: 0,
            };
            if &tag == b"EMBF" {
                let nul = data[body..body_end]
                    .iter()
                    .position(|&b| b == 0)
                    .ok_or_else(|| fmt_err(format!("EMBF at {pos} has no file name")))?;
                c.name = Some(data[body..body + nul].iter().map(|&b| b as char).collect());
                c.name_len = nul + 1;
            } else if &tag == b"INFO" && csize == 1280 {
                let b = &data[body..body_end];
                let version = (b[1073] == 0xBC).then(|| (b[1074], b[1075], b[1076]));
                info = Some(Info {
                    text: cstr_latin1(&b[0..600]),
                    ftp: cstr_latin1(&b[601..801]),
                    web: cstr_latin1(&b[802..1002]),
                    name: cstr_latin1(&b[1003..1067]),
                    version,
                });
            }
            chunks.push(c);
            pos = body_end + (csize & 1);
        }
        if pos != end {
            return Err(fmt_err(format!("trailing bytes at {pos}")));
        }
        Ok(Rbm {
            data,
            chunks,
            info,
            warnings,
        })
    }

    /// Problems that did not stop the parse (a CAT type other than `PRBM`).
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Every chunk, in file order.
    pub fn chunks(&self) -> &[Chunk] {
        &self.chunks
    }

    /// The INFO chunk, when the mod has a 1280-byte one.
    pub fn info(&self) -> Option<&Info> {
        self.info.as_ref()
    }

    /// The EMBF chunks, in file order.
    pub fn embedded(&self) -> impl Iterator<Item = &Chunk> {
        self.chunks.iter().filter(|c| &c.tag == b"EMBF")
    }

    /// The first EMBF whose name equals `name` ignoring case (the original's lookup).
    pub fn find(&self, name: &str) -> Option<&Chunk> {
        self.embedded().find(|c| {
            c.name
                .as_deref()
                .is_some_and(|n| n.eq_ignore_ascii_case(name))
        })
    }

    /// The two u32 sums that pick the key pages.
    pub fn key_sums(&self) -> [u32; 2] {
        let mut acc = SUM_INIT;
        for c in self.embedded() {
            let n = c.size.min(CLEAR);
            let mut buf = [0x65u8; CLEAR];
            buf[..n].copy_from_slice(&self.data[c.offset..c.offset + n]);
            for (j, a) in acc.iter_mut().enumerate() {
                let s = (0..128).fold(0u32, |s, w| s.wrapping_add(be32(&buf, j * 512 + w * 4)));
                *a = a.wrapping_add(s);
            }
        }
        acc
    }

    /// The offsets in `RB20FUL.DAT` of the two key pages.
    pub fn key_pages(&self) -> [u64; 2] {
        self.key_sums()
            .map(|s| DAT_HEADER + u64::from(s) % (DAT_BODY - PAGE as u64))
    }

    /// The descrambled embedded file of `chunk` (without its name).
    pub fn file(&self, chunk: &Chunk, table: &KeyTable) -> Result<Vec<u8>, RbmError> {
        if &chunk.tag != b"EMBF" || self.chunks.get(chunk.ordinal) != Some(chunk) {
            return Err(fmt_err("not a chunk of this mod"));
        }
        let mut p = self.data[chunk.offset..chunk.offset + chunk.size].to_vec();
        descramble(&mut p, chunk.key(), &table.0);
        p.drain(..chunk.name_len);
        Ok(p)
    }

    /// The descrambled embedded file called `name` (case-insensitive, first match).
    pub fn file_by_name(&self, name: &str, table: &KeyTable) -> Result<Vec<u8>, RbmError> {
        let c = self
            .find(name)
            .ok_or_else(|| RbmError::NoSuchFile(name.to_string()))?;
        self.file(c, table)
    }

    /// Maps the mod's samples onto the 46 slots, by name; a slot the mod does not have keeps
    /// the sample of `builtin`. A sample that is not a usable AIFF is an error.
    pub fn load_samples(
        &self,
        table: &KeyTable,
        builtin: &SampleSet,
    ) -> Result<LoadedSamples, RbmError> {
        let mut out = Vec::with_capacity(samples::COUNT);
        let mut from_mod = [false; samples::COUNT];
        for id in samples::FIRST_ID..=samples::LAST_ID {
            let i = (id - samples::FIRST_ID) as usize;
            let name = slot_name(id).expect("id in range");
            match self.find(name) {
                Some(c) => {
                    let bytes = self.file(c, table)?;
                    let s = Sample::from_aiff(&bytes)
                        .map_err(|e| RbmError::Sample(name.to_string(), e.to_string()))?;
                    out.push(Arc::new(s));
                    from_mod[i] = true;
                }
                None => out.push(builtin.shared(id)),
            }
        }
        Ok(LoadedSamples {
            set: SampleSet::from_shared(out),
            from_mod,
        })
    }
}

/// The result of [`Rbm::load_samples`].
#[derive(Debug, Clone)]
pub struct LoadedSamples {
    /// The 46 samples: the mod's, with the built-in ones in the slots it lacks.
    pub set: SampleSet,
    from_mod: [bool; samples::COUNT],
}

impl LoadedSamples {
    /// Whether the sample with resource ID `id` (1000..=1045) comes from the mod.
    pub fn is_from_mod(&self, id: u16) -> bool {
        id.checked_sub(samples::FIRST_ID)
            .and_then(|i| self.from_mod.get(i as usize))
            .copied()
            .unwrap_or(false)
    }
}

/// The 4096-byte key table of one mod.
#[derive(Clone, PartialEq, Eq)]
pub struct KeyTable(Box<[u8; TABLE_LEN]>);

impl fmt::Debug for KeyTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "KeyTable(..)")
    }
}

impl KeyTable {
    /// Reads the mod's two pages from `RB20FUL.DAT` (only 256 + 2 * 2048 bytes are read) after
    /// checking the header magic.
    pub fn read<R: Read + Seek>(rbm: &Rbm, dat: &mut R) -> Result<KeyTable, RbmError> {
        let mut head = [0u8; DAT_HEADER as usize];
        dat.seek(SeekFrom::Start(0)).map_err(RbmError::Io)?;
        dat.read_exact(&mut head).map_err(short)?;
        if be32(&head, 0) != DAT_MAGIC {
            return Err(RbmError::KeyFile(format!(
                "bad magic, not the ReBirth CD's RB20FUL.DAT (expected 0x{DAT_MAGIC:08X})"
            )));
        }
        let mut t = Box::new([0u8; TABLE_LEN]);
        for (k, off) in rbm.key_pages().into_iter().enumerate() {
            dat.seek(SeekFrom::Start(off)).map_err(RbmError::Io)?;
            dat.read_exact(&mut t[k * PAGE..(k + 1) * PAGE])
                .map_err(short)?;
        }
        Ok(KeyTable(t))
    }

    /// A table from its 4096 bytes (tests and tools).
    pub fn from_bytes(b: &[u8]) -> Result<KeyTable, RbmError> {
        let a: [u8; TABLE_LEN] = b.try_into().map_err(|_| RbmError::KeyTable)?;
        Ok(KeyTable(Box::new(a)))
    }
}

fn short(e: io::Error) -> RbmError {
    if e.kind() == io::ErrorKind::UnexpectedEof {
        RbmError::KeyFile("the file is too short".into())
    } else {
        RbmError::Io(e)
    }
}

/// Undoes the scrambling of one EMBF payload, in place (DLL `sub_100163D0`).
fn descramble(p: &mut [u8], key: u8, table: &[u8; TABLE_LEN]) {
    if p.len() <= CLEAR {
        return;
    }
    let period = (17 * usize::from(key)) % 0x785 + 123;
    let base = usize::from(key & 1) << 11;
    for (n, b) in p[CLEAR..].iter_mut().enumerate() {
        *b = b.wrapping_add(table[base + n % period]);
    }
}

/// The inverse of the descrambling (what the mod packer did), for tests and tools.
pub fn scramble(p: &mut [u8], key: u8, table: &KeyTable) {
    if p.len() <= CLEAR {
        return;
    }
    let period = (17 * usize::from(key)) % 0x785 + 123;
    let base = usize::from(key & 1) << 11;
    for (n, b) in p[CLEAR..].iter_mut().enumerate() {
        *b = b.wrapping_sub(table.0[base + n % period]);
    }
}
