//! The ReBirth CD image (`--iso`): `RB20FUL.DAT`, and the files inside the installer
//! (task-065, docs/rbm.md 16).
//!
//! An ISO 9660 image (Joliet names when the image has them) is read directly, in place; no file
//! is extracted to disk and no Windows code runs. `Install ReBirth RB-338.exe` is a Wise for
//! Windows installer whose payload is Microsoft CAB archives (LZX) appended to the exe; they are
//! found by scanning the exe for `MSCF` headers that fit the file, and read with the `cab` crate.
//! The 2001 reissue CD is the only edition known: another image is reported, with what is
//! missing, and the two files `rb mod pack-standard` reads are checked by SHA-256.

use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// The installer at the root of the CD.
pub const INSTALLER: &str = "Install ReBirth RB-338.exe";
/// The key file at the root of the CD.
pub const DAT: &str = "RB20FUL.DAT";
/// ReBirth.dll of the 2001 reissue CD's installer (identical to `original/ReBirth.dll`).
pub const DLL_SHA256: &str = "4c3a6441a2cd50f797d99b1c5b6cf0aaf198c7ad8ca2648196b530c2e3e58b12";
/// Rebirth.exe of the same installer (docs/rbm.md 13.1).
pub const EXE_SHA256: &str = "e69fef4153856f4ad985797a3f312a0027633c00f835c3348415b031bbd87004";

const SECTOR: u64 = 2048;
/// A directory is read whole; a bigger one is not a CD's.
const MAX_DIR: u64 = 16 << 20;

/// One directory entry of the image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    /// First byte of the data in the image.
    pub offset: u64,
    pub size: u64,
}

/// A byte range of the image as a `Read + Seek` of its own. It owns a handle to the image file
/// and seeks it before every read, so windows do not disturb each other.
pub struct Window {
    file: File,
    start: u64,
    len: u64,
    pos: u64,
}

impl Read for Window {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let left = self.len.saturating_sub(self.pos);
        let n = buf.len().min(left.min(usize::MAX as u64) as usize);
        if n == 0 {
            return Ok(0);
        }
        self.file.seek(SeekFrom::Start(self.start + self.pos))?;
        let n = self.file.read(&mut buf[..n])?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for Window {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        let p = match to {
            SeekFrom::Start(p) => Some(p),
            SeekFrom::End(d) => self.len.checked_add_signed(d),
            SeekFrom::Current(d) => self.pos.checked_add_signed(d),
        };
        self.pos = p.ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "seek before the start")
        })?;
        Ok(self.pos)
    }
}

impl Window {
    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The whole range.
    pub fn read_all(&mut self) -> std::io::Result<Vec<u8>> {
        self.pos = 0;
        // no pre-allocation: the size comes from the image
        let mut v = Vec::new();
        let len = self.len;
        self.by_ref().take(len).read_to_end(&mut v)?;
        if (v.len() as u64) < len {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!("the image ends after {} of {len} bytes", v.len()),
            ));
        }
        Ok(v)
    }
}

/// An ISO 9660 image: the Joliet tree when there is one, else the primary one.
pub struct Iso {
    file: File,
    block: u64,
    root: (u64, u64),
    joliet: bool,
}

fn le32(b: &[u8], at: usize) -> u64 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]) as u64
}

impl Iso {
    pub fn open(path: &Path) -> Result<Iso, String> {
        let mut file =
            File::open(path).map_err(|e| format!("{}: cannot read: {e}", path.display()))?;
        let not_iso = |why: &str| {
            format!(
                "{}: not an ISO 9660 image ({why}); rb reads an .iso of the CD with 2048-byte \
                 sectors",
                path.display()
            )
        };
        let mut primary: Option<[u8; 2048]> = None;
        let mut joliet: Option<[u8; 2048]> = None;
        let mut sector = 16u64;
        loop {
            let mut d = [0u8; 2048];
            file.seek(SeekFrom::Start(sector * SECTOR))
                .and_then(|_| file.read_exact(&mut d))
                .map_err(|_| {
                    if sector == 16 {
                        not_iso("too short")
                    } else {
                        not_iso("no volume descriptor terminator")
                    }
                })?;
            if &d[1..6] != b"CD001" {
                return Err(not_iso("no CD001 volume descriptor at sector 16"));
            }
            match d[0] {
                1 if primary.is_none() => primary = Some(d),
                2 if matches!(&d[88..91], b"%/@" | b"%/C" | b"%/E") => joliet = Some(d),
                255 => break,
                _ => {}
            }
            sector += 1;
            if sector > 16 + 32 {
                return Err(not_iso("no volume descriptor terminator"));
            }
        }
        let (d, is_joliet) = match (joliet, primary) {
            (Some(d), _) => (d, true),
            (None, Some(d)) => (d, false),
            _ => return Err(not_iso("no primary volume descriptor")),
        };
        let block = u16::from_le_bytes([d[128], d[129]]) as u64;
        if block != SECTOR {
            return Err(not_iso(&format!("logical block size {block}")));
        }
        Ok(Iso {
            file,
            block,
            root: (le32(&d, 158) * block, le32(&d, 166)),
            joliet: is_joliet,
        })
    }

    fn name(&self, raw: &[u8]) -> String {
        if self.joliet {
            let u: Vec<u16> = (0..raw.len() / 2)
                .map(|i| u16::from_be_bytes([raw[2 * i], raw[2 * i + 1]]))
                .collect();
            let s = String::from_utf16_lossy(&u);
            s.split(';').next().unwrap_or("").to_string()
        } else {
            let s: String = raw.iter().map(|&b| b as char).collect();
            let s = s.split(';').next().unwrap_or("");
            s.strip_suffix('.').unwrap_or(s).to_string()
        }
    }

    /// The entries of the directory at `(offset, size)`, without `.` and `..`.
    fn read_dir(&mut self, (offset, size): (u64, u64)) -> Result<Vec<Entry>, String> {
        if size > MAX_DIR {
            return Err(format!("directory of {size} bytes"));
        }
        let mut buf = vec![0u8; size as usize];
        self.file
            .seek(SeekFrom::Start(offset))
            .and_then(|_| self.file.read_exact(&mut buf))
            .map_err(|e| format!("cannot read a directory: {e}"))?;
        let mut v = Vec::new();
        let mut at = 0usize;
        while at < buf.len() {
            let len = buf[at] as usize;
            if len == 0 {
                // records do not cross a sector: the rest of it is padding
                at = (at / self.block as usize + 1) * self.block as usize;
                continue;
            }
            if len < 34 || at + len > buf.len() || 33 + buf[at + 32] as usize > len {
                return Err("a damaged directory record".into());
            }
            let r = &buf[at..at + len];
            at += len;
            let raw = &r[33..33 + r[32] as usize];
            if raw == [0] || raw == [1] {
                continue;
            }
            if r[25] & 0x80 != 0 {
                return Err(format!(
                    "{}: a multi-extent file, which rb does not read",
                    self.name(raw)
                ));
            }
            v.push(Entry {
                name: self.name(raw),
                is_dir: r[25] & 2 != 0,
                offset: le32(r, 2) * self.block,
                size: le32(r, 10),
            });
        }
        Ok(v)
    }

    /// The entries of the root directory.
    pub fn root_entries(&mut self) -> Result<Vec<Entry>, String> {
        self.read_dir(self.root)
    }

    /// The entry at `path` (`/` separated, names compared without case), or `None`.
    pub fn find(&mut self, path: &str) -> Result<Option<Entry>, String> {
        let mut dir = self.root;
        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        for (i, part) in parts.iter().enumerate() {
            let Some(e) = self
                .read_dir(dir)?
                .into_iter()
                .find(|e| e.name.eq_ignore_ascii_case(part))
            else {
                return Ok(None);
            };
            if i + 1 == parts.len() {
                return Ok(Some(e));
            }
            if !e.is_dir {
                return Ok(None);
            }
            dir = (e.offset, e.size);
        }
        Ok(None)
    }

    /// The data of a file entry.
    pub fn window(&self, e: &Entry) -> Result<Window, String> {
        Ok(Window {
            file: self
                .file
                .try_clone()
                .map_err(|e| format!("cannot reopen the image: {e}"))?,
            start: e.offset,
            len: e.size,
            pos: 0,
        })
    }
}

/// The ReBirth CD image.
pub struct CdImage {
    iso: Iso,
    path: PathBuf,
}

impl CdImage {
    /// Opens the image and checks that it looks like the ReBirth CD: `RB20FUL.DAT` and the
    /// installer at its root.
    pub fn open(path: &Path) -> Result<CdImage, String> {
        let mut iso = Iso::open(path)?;
        let root = iso.root_entries()?;
        let has = |n: &str| {
            root.iter()
                .any(|e| !e.is_dir && e.name.eq_ignore_ascii_case(n))
        };
        let missing: Vec<&str> = [DAT, INSTALLER].into_iter().filter(|n| !has(n)).collect();
        if !missing.is_empty() {
            let mut names: Vec<&str> = root.iter().map(|e| e.name.as_str()).collect();
            names.sort();
            return Err(format!(
                "{}: not the ReBirth 2.01 CD (the 2001 reissue is the only edition rb knows): \
                 no {} at the root of the image, which holds: {}",
                path.display(),
                missing.join(" and "),
                names.join(", ")
            ));
        }
        Ok(CdImage {
            iso,
            path: path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn file(&mut self, name: &str) -> Result<Window, String> {
        let e = self
            .iso
            .find(name)?
            .filter(|e| !e.is_dir)
            .ok_or_else(|| format!("{}: no {name} in the image", self.path.display()))?;
        self.iso.window(&e)
    }

    /// `RB20FUL.DAT`, read in place.
    pub fn rb20ful(&mut self) -> Result<Window, String> {
        self.file(DAT)
    }

    /// The installer's CAB archives, read into memory (20 MB).
    pub fn installer(&mut self) -> Result<Installer, String> {
        let data = self
            .file(INSTALLER)?
            .read_all()
            .map_err(|e| format!("{}: cannot read {INSTALLER}: {e}", self.path.display()))?;
        Ok(Installer::new(data))
    }
}

/// A Wise installer exe: the CAB archives appended to it.
pub struct Installer {
    data: Vec<u8>,
    /// `(offset, size)` of each archive.
    cabs: Vec<(usize, usize)>,
}

impl Installer {
    pub fn new(data: Vec<u8>) -> Installer {
        let mut cabs = Vec::new();
        let mut at = 0;
        while let Some(p) = data[at..].windows(4).position(|w| w == b"MSCF") {
            let i = at + p;
            at = i + 4;
            let h = &data[i..];
            // reserved1 is 0, cbCabinet fits the file, the format is 1.3, there are files
            if h.len() >= 36
                && le32(h, 4) == 0
                && h[24] == 3
                && h[25] == 1
                && u16::from_le_bytes([h[28], h[29]]) > 0
            {
                let size = le32(h, 8) as usize;
                if (36..=h.len()).contains(&size) {
                    cabs.push((i, size));
                    at = i + size;
                }
            }
        }
        Installer { data, cabs }
    }

    fn open(&self, (off, size): (usize, usize)) -> Option<cab::Cabinet<Cursor<&[u8]>>> {
        cab::Cabinet::new(Cursor::new(&self.data[off..off + size])).ok()
    }

    /// The names of all files of all archives that open.
    pub fn names(&self) -> Vec<String> {
        let mut v = Vec::new();
        for &c in &self.cabs {
            if let Some(cab) = self.open(c) {
                for folder in cab.folder_entries() {
                    v.extend(folder.file_entries().map(|f| f.name().to_string()));
                }
            }
        }
        v
    }

    /// The file `name` (without case) of the first archive that has it.
    pub fn read(&self, name: &str) -> Result<Vec<u8>, String> {
        for &c in &self.cabs {
            let Some(mut cab) = self.open(c) else {
                continue;
            };
            let found = cab
                .folder_entries()
                .flat_map(|f| f.file_entries())
                .map(|f| f.name().to_string())
                .find(|n| n.eq_ignore_ascii_case(name));
            if let Some(n) = found {
                let mut v = Vec::new();
                cab.read_file(&n)
                    .and_then(|mut r| r.read_to_end(&mut v))
                    .map_err(|e| format!("{INSTALLER}: cannot unpack {n}: {e}"))?;
                return Ok(v);
            }
        }
        Err(format!(
            "{INSTALLER}: no {name} in its {} CAB archives (another edition of the CD?)",
            self.cabs.len()
        ))
    }
}

pub fn sha256_hex(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

/// A file of the installer that must be the 2001 reissue's: an error names the hashes.
pub fn read_checked(inst: &Installer, name: &str, expected: &str) -> Result<Vec<u8>, String> {
    let b = inst.read(name)?;
    let got = sha256_hex(&b);
    if got != expected {
        return Err(format!(
            "{INSTALLER}: {name} has SHA-256 {got}, expected {expected} (the 2001 reissue CD's): \
             another edition of the CD, which rb does not know"
        ));
    }
    Ok(b)
}

/// The shipped mods in the installer, by name.
pub fn shipped_mods(inst: &Installer) -> Vec<String> {
    let mut v: Vec<String> = inst
        .names()
        .into_iter()
        .filter(|n| n.to_ascii_lowercase().ends_with(".rbm"))
        .collect();
    v.sort();
    v
}
