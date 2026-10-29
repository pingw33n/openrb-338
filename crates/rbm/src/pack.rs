//! Packing and unpacking a mod (task-097, docs/rbm.md section 15): a recrypted mod as a folder
//! of its files plus a `mod.toml`, and back, byte for byte.
//!
//! [`unpack`] decrypts every embedded file with its reader's key and describes the rest in a
//! [`Manifest`]: the INFO fields, the HEAD (when it is not the usual one) and the file order.
//! [`pack`] lays the CAT out in the clear (HEAD, the files in order, INFO) and passes it through
//! [`crate::recrypt`] with an all-zero table (the identity), exactly as the standard mod is built,
//! so the result is a recrypted mod: the key pad after HEAD, every file scrambled against the
//! embedded window under its ordinal.

use std::collections::HashSet;

use crate::{Info, KeyTable, MAX_EMBF, PAD_NAME, Rbm, RbmError, TABLE_LEN, recrypt};

/// The size of INFO (docs/rbm.md section 2).
pub const INFO_LEN: usize = 1280;

/// The name of the manifest file in an unpacked folder.
pub const MANIFEST_NAME: &str = "mod.toml";

/// The first bytes of `HEAD` in every shipped mod (nothing reads them; docs/rbm.md section 2),
/// then the copyright line they carry.
const HEAD_LEAD: [u8; 9] = [0x5B, 0x35, 0xB3, 0x5B, 0xBC, 0x01, 0x00, 0x00, 0x00];
const HEAD_TEXT: &str = "(c)1998 Propellerhead Software, all rights reserved.";

/// The HEAD of every shipped mod (256 bytes), which a manifest without `head` gets.
pub fn standard_head() -> Vec<u8> {
    let mut head = vec![0u8; 256];
    head[..HEAD_LEAD.len()].copy_from_slice(&HEAD_LEAD);
    head[HEAD_LEAD.len()..HEAD_LEAD.len() + HEAD_TEXT.len()].copy_from_slice(HEAD_TEXT.as_bytes());
    head
}

fn err(m: impl Into<String>) -> RbmError {
    RbmError::Pack(m.into())
}

/// One embedded file of the manifest: its name in the mod and its file name in the folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub path: String,
}

/// What `mod.toml` holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub info: Info,
    /// The whole INFO, when the fields cannot reproduce it (a mod with bytes between or after
    /// its fields); it then wins over `info`.
    pub info_raw: Option<Vec<u8>>,
    /// The HEAD payload; `None` is [`standard_head`].
    pub head: Option<Vec<u8>>,
    /// The embedded files in chunk order (the pad is not listed).
    pub files: Vec<Entry>,
}

/// An unpacked mod: the manifest and the files' bytes, in the manifest's order.
#[derive(Debug, Clone)]
pub struct Unpacked {
    pub manifest: Manifest,
    pub files: Vec<Vec<u8>>,
}

fn latin1(what: &str, s: &str, max: usize) -> Result<Vec<u8>, RbmError> {
    let b: Vec<u8> = s
        .chars()
        .map(|c| {
            u8::try_from(u32::from(c)).map_err(|_| err(format!("{what}: {c:?} is not Latin-1")))
        })
        .collect::<Result<_, _>>()?;
    if b.len() > max {
        return Err(err(format!(
            "{what} is {} bytes, the field holds {max}",
            b.len()
        )));
    }
    if b.contains(&0) {
        return Err(err(format!("{what} has a NUL")));
    }
    Ok(b)
}

/// The 1280 bytes of INFO for these fields. A field is NUL-terminated, so it holds one byte less
/// than its space: description 599, ftp 199, web 199, name 63.
pub fn info_bytes(info: &Info) -> Result<[u8; INFO_LEN], RbmError> {
    let mut b = [0u8; INFO_LEN];
    let mut put = |what: &str, at: usize, s: &str, max: usize| -> Result<(), RbmError> {
        let v = latin1(what, s, max)?;
        b[at..at + v.len()].copy_from_slice(&v);
        Ok(())
    };
    put("the description", 0, &info.text, 599)?;
    put("the ftp URL", 601, &info.ftp, 199)?;
    put("the web URL", 802, &info.web, 199)?;
    put("the name", 1003, &info.name, 63)?;
    if info.name.is_empty() {
        return Err(err("the name is empty (the original drops such a mod)"));
    }
    if let Some((a, c, d)) = info.version {
        b[1073..1077].copy_from_slice(&[0xBC, a, c, d]);
    }
    Ok(b)
}

/// The tail of the CAT: INFO as the manifest says it.
fn info_of(m: &Manifest) -> Result<[u8; INFO_LEN], RbmError> {
    match &m.info_raw {
        Some(raw) => raw
            .as_slice()
            .try_into()
            .map_err(|_| err(format!("info_raw is {} bytes, not {INFO_LEN}", raw.len()))),
        None => info_bytes(&m.info),
    }
}

fn push_chunk(out: &mut Vec<u8>, tag: &[u8; 4], payload: &[u8]) {
    out.extend_from_slice(tag);
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        out.push(0);
    }
}

/// Builds a recrypted mod from a HEAD payload, files in order and an INFO. At most
/// [`MAX_EMBF`] - 1 files (the key pad takes an entry), none named [`PAD_NAME`]; the result is
/// parsed and checked to load without `RB20FUL.DAT`.
pub fn pack_parts(
    head: &[u8],
    files: &[(&str, &[u8])],
    info: &[u8; INFO_LEN],
) -> Result<Vec<u8>, RbmError> {
    if files.len() + 1 > MAX_EMBF {
        return Err(err(format!(
            "{} files and the key pad are more than the original's {MAX_EMBF} embedded files \
             (OB-14)",
            files.len()
        )));
    }
    let mut out = b"CAT \0\0\0\0PRBM".to_vec();
    push_chunk(&mut out, b"HEAD", head);
    for (name, data) in files {
        if name.eq_ignore_ascii_case(PAD_NAME) {
            return Err(err(format!(
                "a file named {PAD_NAME} is the key pad's name"
            )));
        }
        let mut p = latin1(&format!("the file name {name:?}"), name, 260)?;
        if p.is_empty() {
            return Err(err("a file has an empty name"));
        }
        p.push(0);
        p.extend_from_slice(data);
        push_chunk(&mut out, b"EMBF", &p);
    }
    push_chunk(&mut out, b"INFO", info);
    let size = u32::try_from(out.len() - 8).map_err(|_| err("the mod would be over 4 GiB"))?;
    out[4..8].copy_from_slice(&size.to_be_bytes());

    // In the clear: a zero table descrambles to the bytes as they are.
    let clear = Rbm::parse(out)?;
    let zero = KeyTable::from_bytes(&[0u8; TABLE_LEN])?;
    let bytes = recrypt(&clear, &zero)?;
    let back = Rbm::parse(bytes.clone())?;
    if !KeyTable::embedded(&back).is_some_and(|t| back.slots_covered(&t) && back.all_covered(&t)) {
        return Err(err(
            "internal: the packed mod does not load without RB20FUL.DAT",
        ));
    }
    Ok(bytes)
}

/// [`pack_parts`] for a manifest and the files' bytes in its order.
pub fn pack(m: &Manifest, files: &[Vec<u8>]) -> Result<Vec<u8>, RbmError> {
    if files.len() != m.files.len() {
        return Err(err("the manifest and the files do not match"));
    }
    let head = m.head.clone().unwrap_or_else(standard_head);
    let parts: Vec<(&str, &[u8])> = m
        .files
        .iter()
        .zip(files)
        .map(|(e, d)| (e.name.as_str(), d.as_slice()))
        .collect();
    pack_parts(&head, &parts, &info_of(m)?)
}

/// The key two file names clash on: Unicode lower case (`char::to_lowercase`), the one fold
/// that unpack and pack both use. The file system may fold more (macOS normalizes Unicode);
/// unpack writes with `create_new`, so what it folds further is an error, not an overwrite.
pub fn fold(s: &str) -> String {
    s.chars().flat_map(char::to_lowercase).collect()
}

/// A Windows device name: `CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9` (the superscript digits
/// too), in any case, with any extension, and with spaces or dots before the extension.
fn is_device_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or("").trim_end_matches(' ');
    let up = stem.to_uppercase();
    matches!(up.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|p| {
            up.strip_prefix(p).is_some_and(|d| {
                let mut c = d.chars();
                matches!(
                    (c.next(), c.next()),
                    (Some('1'..='9' | '\u{b9}' | '\u{b2}' | '\u{b3}'), None)
                )
            })
        })
}

/// Whether `name` is a plain file name on every system: not empty, not `.` or `..`, at most 200
/// bytes, none of `/ \ : * ? " < > |`, no control character (NUL included), no trailing dot or
/// space, not a Windows device name and not `mod.toml` (in any case). The one rule for the
/// paths unpack writes and pack reads.
pub fn safe_path(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && name.len() <= 200
        && fold(name) != MANIFEST_NAME
        && !name.chars().any(|c| {
            c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
        && !name.ends_with(['.', ' '])
        && !is_device_name(name)
}

/// The folder file name for the embedded file `name` as the `index`th file: the name itself when
/// it is safe and unused, else `NNN_` and the name with its unsafe characters replaced, with the
/// first index that is unused. `used` holds the [`fold`]ed paths taken; the choice is added.
fn pick_path(name: &str, index: usize, used: &mut HashSet<String>) -> String {
    if safe_path(name) && !used.contains(&fold(name)) {
        used.insert(fold(name));
        return name.to_string();
    }
    let mut stem: String = name
        .chars()
        .map(|ch| if safe_path(&ch.to_string()) { ch } else { '_' })
        .take(100)
        .collect();
    let mut n = index;
    loop {
        let mut p = format!("{n:03}_{stem}");
        if !safe_path(&p) {
            // a trailing dot or space, or a stem of dots
            stem = "file".into();
            p = format!("{n:03}_{stem}");
        }
        if used.insert(fold(&p)) {
            return p;
        }
        n += 1;
    }
}

/// Unpacks a recrypted mod; `table` is [`KeyTable::embedded`] of it, covering every file.
/// Allowed layout: HEAD first, then EMBF chunks (the key pad is dropped), INFO last.
pub fn unpack(rbm: &Rbm, table: &KeyTable) -> Result<Unpacked, RbmError> {
    let chunks = rbm.chunks();
    if chunks.first().map(|c| &c.tag) != Some(b"HEAD") {
        return Err(err("the mod does not start with a HEAD chunk"));
    }
    let info_c = match chunks.last() {
        Some(c) if &c.tag == b"INFO" && c.size == INFO_LEN => c,
        _ => return Err(err("the mod does not end with a 1280-byte INFO chunk")),
    };
    let mut files = Vec::new();
    let mut data = Vec::new();
    let mut used: HashSet<String> = HashSet::new();
    for c in &chunks[1..chunks.len() - 1] {
        if &c.tag != b"EMBF" {
            return Err(err(format!(
                "chunk {} is {:?}, not EMBF: unpack keeps only HEAD, the files and INFO",
                c.ordinal,
                String::from_utf8_lossy(&c.tag)
            )));
        }
        let name = c.name.clone().unwrap_or_default();
        if name == PAD_NAME && c.size == crate::CLEAR {
            continue;
        }
        let path = pick_path(&name, files.len(), &mut used);
        data.push(rbm.file(c, table)?);
        files.push(Entry { name, path });
    }
    let raw = rbm.raw_payload(info_c).to_vec();
    let info = rbm
        .info()
        .cloned()
        .ok_or_else(|| err("the mod has no INFO"))?;
    let mut manifest = Manifest {
        info,
        info_raw: None,
        head: None,
        files,
    };
    if info_of(&manifest).ok().as_ref().map(|b| b.as_slice()) != Some(raw.as_slice()) {
        manifest.info_raw = Some(raw);
    }
    let head = rbm.raw_payload(&chunks[0]);
    if head != standard_head() {
        manifest.head = Some(head.to_vec());
    }
    Ok(Unpacked {
        manifest,
        files: data,
    })
}

pub fn to_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn from_hex(s: &str) -> Result<Vec<u8>, RbmError> {
    let s: Vec<u8> = s.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if !s.len().is_multiple_of(2) {
        return Err(err("a hex string has an odd length"));
    }
    s.chunks(2)
        .map(|p| {
            std::str::from_utf8(p)
                .ok()
                .and_then(|h| u8::from_str_radix(h, 16).ok())
                .ok_or_else(|| err("bad hex digit"))
        })
        .collect()
}

fn q(s: &str) -> String {
    toml::Value::String(s.to_string()).to_string()
}

impl Manifest {
    /// The text of `mod.toml`.
    pub fn to_toml(&self) -> String {
        let i = &self.info;
        let mut s = String::from(
            "# orb mod unpack: edit the fields and the files, then `orb mod pack DIR -o MOD.rbm`.\n\
             # files: the embedded files in chunk order (the order matters: the first of a name\n\
             # is the one looked up). A plain string is both the name in the mod and the file\n\
             # name here; { name, path } gives them apart.\n",
        );
        s += &format!("name = {}\n", q(&i.name));
        if let Some((a, b, c)) = i.version {
            s += &format!("version = \"{a}.{b}.{c}\"\n");
        }
        s += &format!("description = {}\n", q(&i.text));
        s += &format!("ftp = {}\n", q(&i.ftp));
        s += &format!("web = {}\n", q(&i.web));
        if let Some(h) = &self.head {
            s += &format!("head = \"{}\"\n", to_hex(h));
        }
        if let Some(r) = &self.info_raw {
            s += &format!("info_raw = \"{}\"\n", to_hex(r));
        }
        s += "files = [\n";
        for e in &self.files {
            if e.name == e.path {
                s += &format!("    {},\n", q(&e.name));
            } else {
                s += &format!("    {{ name = {}, path = {} }},\n", q(&e.name), q(&e.path));
            }
        }
        s += "]\n";
        s
    }

    /// Reads `mod.toml`.
    pub fn from_toml(text: &str) -> Result<Manifest, RbmError> {
        let t: toml::Table = text
            .parse()
            .map_err(|e| err(format!("{MANIFEST_NAME}: {e}")))?;
        const KEYS: [&str; 8] = [
            "name",
            "version",
            "description",
            "ftp",
            "web",
            "head",
            "info_raw",
            "files",
        ];
        if let Some(k) = t.keys().find(|k| !KEYS.contains(&k.as_str())) {
            return Err(err(format!("{MANIFEST_NAME}: unknown key {k:?}")));
        }
        let string = |k: &str, required: bool| -> Result<Option<String>, RbmError> {
            match t.get(k) {
                Some(toml::Value::String(s)) => Ok(Some(s.clone())),
                Some(_) => Err(err(format!("{MANIFEST_NAME}: {k} must be a string"))),
                None if required => Err(err(format!("{MANIFEST_NAME}: {k} is missing"))),
                None => Ok(None),
            }
        };
        let version = match string("version", false)? {
            None => None,
            Some(v) => {
                let p: Vec<u8> = v
                    .split('.')
                    .map(|x| x.trim().parse::<u8>())
                    .collect::<Result<_, _>>()
                    .map_err(|_| err(format!("{MANIFEST_NAME}: version {v:?} is not A.B.C")))?;
                match p[..] {
                    [a, b, c] => Some((a, b, c)),
                    _ => return Err(err(format!("{MANIFEST_NAME}: version {v:?} is not A.B.C"))),
                }
            }
        };
        let hex = |k: &str| string(k, false)?.map(|h| from_hex(&h)).transpose();
        let Some(toml::Value::Array(list)) = t.get("files") else {
            return Err(err(format!("{MANIFEST_NAME}: files must be a list")));
        };
        let mut files = Vec::new();
        for v in list {
            let e = match v {
                toml::Value::String(s) => Entry {
                    name: s.clone(),
                    path: s.clone(),
                },
                toml::Value::Table(e) => match (e.get("name"), e.get("path")) {
                    (Some(toml::Value::String(n)), Some(toml::Value::String(p))) => Entry {
                        name: n.clone(),
                        path: p.clone(),
                    },
                    _ => return Err(err(format!("{MANIFEST_NAME}: a file needs name and path"))),
                },
                _ => return Err(err(format!("{MANIFEST_NAME}: bad entry in files"))),
            };
            files.push(e);
        }
        Ok(Manifest {
            info: Info {
                text: string("description", false)?.unwrap_or_default(),
                ftp: string("ftp", false)?.unwrap_or_default(),
                web: string("web", false)?.unwrap_or_default(),
                name: string("name", true)?.unwrap_or_default(),
                version,
            },
            info_raw: hex("info_raw")?,
            head: hex("head")?,
            files,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn big(seed: u8, n: usize) -> Vec<u8> {
        (0..n)
            .map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed))
            .collect()
    }

    fn info() -> Info {
        Info {
            text: "line one\r\nline two".into(),
            ftp: String::new(),
            web: "http://x/".into(),
            name: "Test".into(),
            version: Some((1, 0, 0)),
        }
    }

    fn sample() -> Vec<u8> {
        let i = info_bytes(&info()).unwrap();
        let (b1, b2) = (big(1, 5000), big(2, 3001));
        let parts: Vec<(&str, &[u8])> = vec![
            ("a.bin", &[1, 2, 3]),
            ("TR808BD.aif", &b1),
            ("b.jpg", &b2),
            ("a.bin", &[9]),
        ];
        pack_parts(&standard_head(), &parts, &i).unwrap()
    }

    fn unpacked(bytes: &[u8]) -> Unpacked {
        let r = Rbm::parse(bytes.to_vec()).unwrap();
        let t = KeyTable::embedded(&r).unwrap();
        unpack(&r, &t).unwrap()
    }

    #[test]
    fn round_trip_is_byte_identical() {
        let m = sample();
        let u = unpacked(&m);
        assert_eq!(u.manifest.files.len(), 4);
        assert_eq!(u.manifest.head, None);
        assert_eq!(u.manifest.info_raw, None);
        assert_eq!(u.manifest.info, info());
        // the duplicate name needs a path of its own
        assert_eq!(u.manifest.files[3].name, "a.bin");
        assert_ne!(u.manifest.files[3].path, "a.bin");
        assert_eq!(pack(&u.manifest, &u.files).unwrap(), m);
        // through the text as well
        let text = u.manifest.to_toml();
        let back = Manifest::from_toml(&text).unwrap();
        assert_eq!(back, u.manifest);
    }

    #[test]
    fn odd_head_and_info_bytes_survive() {
        let mut i = info_bytes(&info()).unwrap();
        i[1200] = 7;
        let m = pack_parts(&[1, 2, 3], &[("f", &[5])], &i).unwrap();
        let u = unpacked(&m);
        assert_eq!(u.manifest.head, Some(vec![1, 2, 3]));
        assert_eq!(u.manifest.info_raw.as_deref(), Some(&i[..]));
        let back = Manifest::from_toml(&u.manifest.to_toml()).unwrap();
        assert_eq!(pack(&back, &u.files).unwrap(), m);
    }

    #[test]
    fn edited_file_reads_back() {
        let m = sample();
        let mut u = unpacked(&m);
        u.files[1] = big(77, 4321);
        let out = pack(&u.manifest, &u.files).unwrap();
        let r = Rbm::parse(out).unwrap();
        let t = KeyTable::embedded(&r).unwrap();
        assert!(r.slots_covered(&t));
        assert_eq!(r.file_by_name("TR808BD.aif", &t).unwrap(), big(77, 4321));
    }

    #[test]
    fn safe_paths() {
        for ok in [
            "a.bin",
            "12017.JPG",
            "\u{c9}.jpg",
            "COM0",
            "COM10",
            "CONX",
            "a b",
            "x.y.z",
        ] {
            assert!(safe_path(ok), "{ok}");
        }
        let long = "x".repeat(201);
        for bad in [
            "",
            ".",
            "..",
            "../x",
            "a/b",
            "/abs",
            "a\\b",
            "C:x",
            "CON",
            "con",
            "nul.aif",
            "CON .aif",
            "Con..txt",
            "COM1",
            "lpt9.x",
            "COM\u{b9}",
            "a.",
            "a ",
            "x\u{1}y",
            "x\0y",
            "mod.toml",
            "MOD.TOML",
            "a*b",
            &long,
        ] {
            assert!(!safe_path(bad), "{bad:?}");
        }
        assert_eq!(fold("\u{c9}A"), fold("\u{e9}a"));
    }

    #[test]
    fn picked_paths_never_clash() {
        let mut used = HashSet::new();
        let a = pick_path("001_a_b", 0, &mut used);
        let b = pick_path("a/b", 1, &mut used);
        let c = pick_path("A/B", 1, &mut used);
        assert_eq!(a, "001_a_b");
        assert!(safe_path(&b) && safe_path(&c));
        assert_eq!(used.len(), 3);
        assert_eq!(pick_path("\u{c9}.jpg", 3, &mut used), "\u{c9}.jpg");
        assert_ne!(pick_path("\u{e9}.jpg", 4, &mut used), "\u{e9}.jpg");
        assert!(safe_path(&pick_path("a.", 5, &mut used)));
    }

    #[test]
    fn errors() {
        let i = info_bytes(&info()).unwrap();
        let many: Vec<(&str, &[u8])> = (0..200).map(|_| ("f", &[1u8][..])).collect();
        assert!(pack_parts(&standard_head(), &many, &i).is_err());
        let ok: Vec<(&str, &[u8])> = (0..199).map(|_| ("f", &[1u8][..])).collect();
        assert!(pack_parts(&standard_head(), &ok, &i).is_ok());
        let mut f = info();
        f.name = "x".repeat(64);
        assert!(info_bytes(&f).is_err());
        f.name = "x".repeat(63);
        assert!(info_bytes(&f).is_ok());
        f.text = "x".repeat(600);
        assert!(info_bytes(&f).is_err());
        f = info();
        f.ftp = "x".repeat(200);
        assert!(info_bytes(&f).is_err());
        f = info();
        f.web = "x".repeat(200);
        assert!(info_bytes(&f).is_err());
        f = info();
        f.name = String::new();
        assert!(info_bytes(&f).is_err());
        f = info();
        f.text = "\u{263a}".into();
        assert!(info_bytes(&f).is_err());
        assert!(pack_parts(&standard_head(), &[(PAD_NAME, &[1])], &i).is_err());
        assert!(Manifest::from_toml("name = \"x\"\nfiles = []\nbogus = 1\n").is_err());
        assert!(Manifest::from_toml("files = []\n").is_err());
    }
}
