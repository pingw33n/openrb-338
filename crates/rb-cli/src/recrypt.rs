//! `rb mod recrypt`: one mod or many (task-059, task-064; docs/rbm.md 12.5).
//!
//! The key file is found, read and hashed at most once per run, when the first mod that is not
//! recrypted yet needs it; each mod then reads only its two 2048-byte key pages. A mod that fails
//! does not stop the others. Nothing is written for a mod whose key file or files do not check.

use std::collections::HashSet;
use std::fs::File;
use std::path::{Path, PathBuf};

use crate::mods;
use crate::{Fail, fail};

/// What `rb mod recrypt` was asked.
#[derive(Debug, Clone, Copy)]
pub struct Options<'a> {
    /// `.rbm` files and folders (every `.rbm` directly in a folder).
    pub inputs: &'a [PathBuf],
    /// The mod to write (one input file and a path that is not an existing folder and ends in
    /// `.rbm`), else the folder to write the mods into, under their input names.
    pub output: &'a Path,
    /// `--rb20ful`; see [`mods::find_rb20ful`] for the search.
    pub rb20ful: Option<&'a Path>,
    /// Overwrite existing outputs.
    pub force: bool,
    /// The SHA-256 the key file must have: [`rbm::embedded::DAT_SHA256`] (a test passes
    /// another one for a stand-in key file).
    pub expected_sha256: &'a str,
}

/// What became of one mod.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Rewritten.
    Recrypted {
        chunks_before: usize,
        chunks_after: usize,
        bytes_before: usize,
        bytes_after: usize,
    },
    /// The mod was recrypted already: its files were checked and it was copied unchanged.
    AlreadyRecrypted,
    /// Not written; the reason.
    Failed(String),
}

/// The result of a run.
#[derive(Debug)]
pub struct Report {
    /// Per mod: the input, the output and what happened.
    pub results: Vec<(PathBuf, PathBuf, Outcome)>,
    /// How many times the key file was hashed (0 when no mod needed it, else 1).
    pub dat_hashes: usize,
}

impl Report {
    pub fn failed(&self) -> usize {
        self.results
            .iter()
            .filter(|r| matches!(r.2, Outcome::Failed(_)))
            .count()
    }

    /// The summary: recrypted, already recrypted, failed (with the reasons).
    pub fn summary(&self) -> String {
        let recrypted = self
            .results
            .iter()
            .filter(|r| matches!(r.2, Outcome::Recrypted { .. }))
            .count();
        let already = self
            .results
            .iter()
            .filter(|r| r.2 == Outcome::AlreadyRecrypted)
            .count();
        let failed = self.failed();
        let mut s =
            format!("{recrypted} recrypted, {already} already recrypted (copied), {failed} failed");
        for (input, _, o) in &self.results {
            if let Outcome::Failed(why) = o {
                s.push_str(&format!("\n  failed: {}: {why}", input.display()));
            }
        }
        s
    }
}

/// The key file, found and verified at the first use.
struct Dat<'a> {
    explicit: Option<&'a Path>,
    expected: &'a str,
    first_input: &'a Path,
    opened: Option<Result<File, String>>,
    hashes: usize,
}

impl Dat<'_> {
    fn file(&mut self) -> Result<&mut File, String> {
        if self.opened.is_none() {
            let r = self.open();
            self.opened = Some(r);
        }
        match self.opened.as_mut() {
            Some(Ok(f)) => Ok(f),
            Some(Err(e)) => Err(e.clone()),
            None => unreachable!(),
        }
    }

    fn open(&mut self) -> Result<File, String> {
        let path = mods::find_rb20ful(self.explicit, self.first_input)
            .map_err(|Fail(_, m)| format!("recrypting needs the key file once; {m}"))?;
        let mut f =
            File::open(&path).map_err(|e| format!("{}: cannot read: {e}", path.display()))?;
        eprintln!("rb: checking {} (SHA-256)", path.display());
        self.hashes += 1;
        rbm::dat::verify_dat(&mut f, self.expected)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(f)
    }
}

/// The `.rbm` files of a folder, by name.
fn rbm_files(dir: &Path) -> Result<Vec<PathBuf>, Fail> {
    let rd = std::fs::read_dir(dir)
        .map_err(|e| fail(1, format!("{}: cannot read the folder: {e}", dir.display())))?;
    let mut v: Vec<PathBuf> = rd
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file() && has_rbm_ext(p))
        .collect();
    v.sort();
    if v.is_empty() {
        return Err(fail(1, format!("{}: no .rbm files", dir.display())));
    }
    Ok(v)
}

fn has_rbm_ext(p: &Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("rbm"))
}

fn same_path(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// One mod to do: its input and output, and whether an earlier input has the same output name.
struct Job {
    input: PathBuf,
    output: PathBuf,
    dup: bool,
}

/// The jobs of a run.
fn plan(o: &Options) -> Result<Vec<Job>, Fail> {
    if o.inputs.is_empty() {
        return Err(fail(2, "no input mod"));
    }
    let file_output = !o.output.is_dir() && has_rbm_ext(o.output);
    if file_output && !(o.inputs.len() == 1 && o.inputs[0].is_file()) {
        // Never made a folder: `x.rbm` is a file name, so it takes exactly one existing mod.
        return Err(fail(
            2,
            format!(
                "{} is a file name (it ends in .rbm and is not a folder): it takes one input \
                 mod, which must exist. For several mods or a folder, give an output folder",
                o.output.display()
            ),
        ));
    }
    if file_output {
        if same_path(&o.inputs[0], o.output) {
            return Err(fail(2, "the output is the input"));
        }
        return Ok(vec![Job {
            input: o.inputs[0].clone(),
            output: o.output.to_path_buf(),
            dup: false,
        }]);
    }
    let mut mods = Vec::new();
    for i in o.inputs {
        if i.is_dir() {
            mods.extend(rbm_files(i)?);
        } else {
            mods.push(i.clone());
        }
    }
    for m in &mods {
        let dir = match m.parent() {
            Some(d) if !d.as_os_str().is_empty() => d,
            _ => Path::new("."),
        };
        if o.output.exists() && same_path(dir, o.output) {
            return Err(fail(
                2,
                format!(
                    "the output folder {} is the folder of the input {}",
                    o.output.display(),
                    m.display()
                ),
            ));
        }
    }
    // The output folder is made by the first mod that is written (see `one`), so a run in which
    // nothing passes the checks leaves nothing behind.
    let mut seen = HashSet::new();
    let mut jobs = Vec::new();
    for m in mods {
        let name = m
            .file_name()
            .ok_or_else(|| fail(2, "an input has no name"))?;
        let out = o.output.join(name);
        // A second mod of the same name would overwrite the first: it fails.
        let dup = !seen.insert(name.to_string_lossy().to_lowercase());
        jobs.push(Job {
            input: m,
            output: out,
            dup,
        });
    }
    Ok(jobs)
}

/// Recrypts one mod; the `Err` is the reason it was not written.
fn one(dat: &mut Dat, job: &Job, force: bool) -> Result<Outcome, String> {
    let (input, output) = (&job.input, &job.output);
    if job.dup {
        return Err("another input has the same file name".into());
    }
    if output.exists() && !force {
        return Err(format!(
            "{} exists (--force overwrites it)",
            output.display()
        ));
    }
    let bytes = std::fs::read(input).map_err(|e| format!("cannot read the mod: {e}"))?;
    let rbm = rbm::Rbm::parse(bytes.clone()).map_err(|e| e.to_string())?;
    let embedded = rbm::KeyTable::embedded(&rbm).filter(|t| rbm.all_covered(t));
    let (out, outcome) = if let Some(t) = embedded {
        rbm::dat::validate_files(&rbm, &t).map_err(|e| e.to_string())?;
        (bytes.clone(), Outcome::AlreadyRecrypted)
    } else {
        let f = dat.file()?;
        let table = rbm::KeyTable::read(&rbm, f).map_err(|e| e.to_string())?;
        rbm::dat::validate_files(&rbm, &table).map_err(|e| e.to_string())?;
        let out = rbm::recrypt(&rbm, &table).map_err(|e| e.to_string())?;
        let back = rbm::Rbm::parse(out.clone())
            .map_err(|e| format!("internal: the recrypted mod does not parse: {e}"))?;
        rbm::dat::validate_files(
            &back,
            &rbm::KeyTable::embedded(&back).ok_or("internal: the recrypted mod needs the DAT")?,
        )
        .map_err(|e| format!("internal: the recrypted mod: {e}"))?;
        let o = Outcome::Recrypted {
            chunks_before: rbm.chunks().len(),
            chunks_after: back.chunks().len(),
            bytes_before: bytes.len(),
            bytes_after: out.len(),
        };
        (out, o)
    };
    if let Some(dir) = output.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("{}: cannot create: {e}", dir.display()))?;
    }
    std::fs::write(output, &out).map_err(|e| format!("{}: cannot write: {e}", output.display()))?;
    Ok(outcome)
}

/// Runs `rb mod recrypt`. `Err` is a usage error (no input, the output folder is an input
/// folder); a failing mod is an [`Outcome::Failed`] in the report.
pub fn run(o: &Options) -> Result<Report, Fail> {
    let jobs = plan(o)?;
    let mut dat = Dat {
        explicit: o.rb20ful,
        expected: o.expected_sha256,
        first_input: &jobs[0].input,
        opened: None,
        hashes: 0,
    };
    let mut results = Vec::new();
    for job in &jobs {
        let (input, output) = (&job.input, &job.output);
        let outcome = one(&mut dat, job, o.force).unwrap_or_else(Outcome::Failed);
        match &outcome {
            Outcome::Recrypted {
                chunks_before,
                chunks_after,
                bytes_before,
                bytes_after,
            } => eprintln!(
                "rb: {} -> {}: {chunks_before} -> {chunks_after} chunks, {bytes_before} -> \
                 {bytes_after} bytes",
                input.display(),
                output.display()
            ),
            Outcome::AlreadyRecrypted => eprintln!(
                "rb: {} -> {}: already recrypted, copied unchanged",
                input.display(),
                output.display()
            ),
            Outcome::Failed(why) => eprintln!("rb: {}: failed: {why}", input.display()),
        }
        results.push((input.clone(), output.clone(), outcome));
    }
    Ok(Report {
        results,
        dat_hashes: dat.hashes,
    })
}
