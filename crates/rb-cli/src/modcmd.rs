//! `rb mod unpack` and `rb mod pack` (task-097, docs/rbm.md section 15): the file system side of
//! `rbm::pack`.

use std::collections::HashSet;
use std::io::Write;
use std::path::Path;

use rb_cli::render::read_mod;
use rb_cli::{Fail, fail};
use rbm::pack::{self, MANIFEST_NAME, Manifest};

/// `rb mod unpack`: a recrypted mod into a folder.
pub fn unpack(input: &Path, dir: &Path, force: bool) -> Result<(), Fail> {
    let rbm = read_mod(input)?;
    let table = rbm::KeyTable::embedded(&rbm)
        .filter(|t| rbm.all_covered(t))
        .ok_or_else(|| {
            fail(
                1,
                format!(
                    "{}: the mod is not recrypted. Run `rb mod recrypt --rb20ful RB20FUL.DAT {} \
                     -o OUT.rbm` once and unpack OUT.rbm",
                    input.display(),
                    input.display()
                ),
            )
        })?;
    let u = pack::unpack(&rbm, &table).map_err(|e| fail(1, format!("{}: {e}", input.display())))?;
    if dir.exists() {
        if !dir.is_dir() {
            return Err(fail(1, format!("{}: not a folder", dir.display())));
        }
        let empty = std::fs::read_dir(dir)
            .map_err(|e| fail(1, format!("{}: {e}", dir.display())))?
            .next()
            .is_none();
        if !empty && !force {
            return Err(fail(
                1,
                format!(
                    "{}: not empty (use --force to write into it)",
                    dir.display()
                ),
            ));
        }
    } else {
        std::fs::create_dir_all(dir)
            .map_err(|e| fail(1, format!("{}: cannot create: {e}", dir.display())))?;
    }
    // What the folder held before: with --force a file of that exact name is replaced. A file
    // this run wrote is never replaced: create_new fails on any clash the file system sees
    // (case, Unicode normalization, ...), so no unpacked file overwrites another.
    let before: HashSet<String> = std::fs::read_dir(dir)
        .map_err(|e| fail(1, format!("{}: {e}", dir.display())))?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    let write = |name: &str, bytes: &[u8]| {
        let p = dir.join(name);
        if force && before.contains(name) {
            std::fs::remove_file(&p)
                .map_err(|e| fail(1, format!("{}: cannot replace: {e}", p.display())))?;
        }
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&p)
            .map_err(|e| {
                fail(
                    1,
                    format!(
                        "{}: cannot create (a clash with another file of the mod, or an \
                         existing one): {e}",
                        p.display()
                    ),
                )
            })?;
        f.write_all(bytes)
            .map_err(|e| fail(1, format!("{}: cannot write: {e}", p.display())))
    };
    for (e, data) in u.manifest.files.iter().zip(&u.files) {
        write(&e.path, data)?;
    }
    write(MANIFEST_NAME, u.manifest.to_toml().as_bytes())?;
    eprintln!(
        "rb: {} -> {}: {} files and {MANIFEST_NAME}",
        input.display(),
        dir.display(),
        u.files.len()
    );
    Ok(())
}

/// `rb mod pack`: a folder made by unpack into a recrypted mod.
pub fn pack(dir: &Path, output: &Path) -> Result<(), Fail> {
    let mp = dir.join(MANIFEST_NAME);
    let text = std::fs::read_to_string(&mp)
        .map_err(|e| fail(1, format!("{}: cannot read: {e}", mp.display())))?;
    let m = Manifest::from_toml(&text).map_err(|e| fail(1, format!("{}: {e}", mp.display())))?;

    let mut folded: HashSet<String> = HashSet::new();
    let mut listed: HashSet<&str> = HashSet::new();
    let mut files = Vec::with_capacity(m.files.len());
    for e in &m.files {
        if !pack::safe_path(&e.path) {
            return Err(fail(
                1,
                format!(
                    "{}: bad file path {:?} (not a plain file name: see docs/rbm.md 15.1)",
                    mp.display(),
                    e.path
                ),
            ));
        }
        listed.insert(&e.path);
        if !folded.insert(pack::fold(&e.path)) {
            return Err(fail(
                1,
                format!("{}: {:?} is listed twice", mp.display(), e.path),
            ));
        }
        let p = dir.join(&e.path);
        let data = std::fs::read(&p).map_err(|err| {
            fail(
                1,
                format!(
                    "{}: listed in {MANIFEST_NAME} but cannot be read: {err}",
                    p.display()
                ),
            )
        })?;
        files.push(data);
    }
    // A file the manifest does not list would be lost silently: an error.
    for ent in std::fs::read_dir(dir).map_err(|e| fail(1, format!("{}: {e}", dir.display())))? {
        let name = ent
            .map_err(|e| fail(1, format!("{}: {e}", dir.display())))?
            .file_name()
            .to_string_lossy()
            .into_owned();
        // exact names: "a.bin" next to a listed "A.bin" is a file the manifest does not list
        if name != MANIFEST_NAME && !listed.contains(name.as_str()) {
            return Err(fail(
                1,
                format!(
                    "{}: {name:?} is not listed in {MANIFEST_NAME}; list it or remove it",
                    dir.display()
                ),
            ));
        }
    }
    let bytes = pack::pack(&m, &files).map_err(|e| fail(1, format!("{}: {e}", dir.display())))?;
    std::fs::write(output, &bytes)
        .map_err(|e| fail(1, format!("{}: cannot write: {e}", output.display())))?;
    eprintln!(
        "rb: {} -> {}: {} files, {} bytes",
        dir.display(),
        output.display(),
        files.len(),
        bytes.len()
    );
    Ok(())
}
