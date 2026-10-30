//! The version string of the app (task-102, task-133): `<version> (<git>)`, the version being
//! the UTC build time as `YYYY.MMDD.HHMM`, for example `2026.1028.1132 (7bd8876)`. This file
//! has no dependencies: `build.rs` includes it (`#[path]`) to compute the string, and the
//! library tests it.
#![allow(dead_code)]

/// The git state of a build.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Git<'a> {
    /// The short hash of HEAD.
    pub hash: &'a str,
    /// Whether the work tree had uncommitted changes.
    pub dirty: bool,
    /// Whether the build is of the main branch.
    pub main: bool,
}

/// Whether the build is of the main branch. In order of precedence:
/// - `override_` (`ORB_BUILD_BRANCH`, set by release builds): main or anything else;
/// - the branch git reports (`git rev-parse --abbrev-ref HEAD`), unless it is a detached HEAD
///   (`HEAD`) or absent;
/// - `github_ref_name` (`GITHUB_REF_NAME`; a tag build in GitHub Actions is detached);
/// - otherwise not main.
pub fn is_main(
    override_: Option<&str>,
    git_branch: Option<&str>,
    github_ref_name: Option<&str>,
) -> bool {
    fn non_empty(s: Option<&str>) -> Option<&str> {
        s.map(str::trim).filter(|s| !s.is_empty())
    }
    if let Some(b) = non_empty(override_) {
        return b == "main";
    }
    if let Some(b) = non_empty(git_branch).filter(|b| *b != "HEAD") {
        return b == "main";
    }
    non_empty(github_ref_name) == Some("main")
}

/// The version number of the Unix time `secs`: `YYYY.MMDD.HHMM` in UTC, to the minute and
/// zero-padded (task-133). It is what `scripts/version.sh` prints, the release tag is `v` and
/// it, and the macOS bundle has it. Later builds compare higher, as text and per component.
pub fn version_number(secs: i64) -> String {
    let [y, m, d, h, mi, _] = utc(secs);
    format!("{y:04}.{m:02}{d:02}.{h:02}{mi:02}")
}

/// The Unix time `secs` as UTC `[year, month, day, hour, minute, second]` (the crash log's
/// time, task-157).
pub fn utc(secs: i64) -> [i64; 6] {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // civil from days (H. Hinnant), days since 1970-01-01
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    [y, m, d, rem / 3600, rem % 3600 / 60, rem % 60]
}

/// The version string: the version number and the git state, `(unknown)` with no git.
pub fn version_string(secs: i64, git: Option<Git>) -> String {
    let g = match git {
        None => "unknown".to_string(),
        Some(g) => format!(
            "{}{}{}",
            if g.main { "" } else { "dev-" },
            g.hash,
            if g.dirty { "-dirty" } else { "" }
        ),
    };
    format!("{} ({g})", version_number(secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2026-10-28 11:32:11 UTC
    const T: i64 = 1_793_187_131;

    fn git(main: bool, dirty: bool) -> Option<Git<'static>> {
        Some(Git {
            hash: "7bd8876",
            dirty,
            main,
        })
    }

    #[test]
    fn the_version_number_is_utc_to_the_minute() {
        assert_eq!(version_number(0), "1970.0101.0000");
        assert_eq!(version_number(T), "2026.1028.1132");
        // seconds are dropped, not rounded
        assert_eq!(version_number(T - 12), "2026.1028.1131");
        assert_eq!(version_number(T - 11), "2026.1028.1132");
        // the example of the decision (2026-10-28), a leap day and the end of a year
        assert_eq!(version_number(1_793_215_800), "2026.1028.1930");
        assert_eq!(version_number(1_709_164_799), "2024.0228.2359");
        assert_eq!(version_number(951_782_400), "2000.0229.0000");
        // 2100 is no leap year
        assert_eq!(version_number(4_107_542_400), "2100.0301.0000");
        assert_eq!(version_number(1_709_164_800), "2024.0229.0000");
        assert_eq!(version_number(1_709_251_200), "2024.0301.0000");
        assert_eq!(version_number(1_767_225_599), "2025.1231.2359");
    }

    #[test]
    fn later_builds_compare_higher() {
        let parts = |s: &str| -> Vec<u32> { s.split('.').map(|p| p.parse().unwrap()).collect() };
        // zero-padded: a later time is higher as text and per component
        let times = [
            0,
            1_000_000_000,
            T - 7200,
            T,
            T + 60,
            T + 40 * 86_400,
            4_107_542_400,
        ];
        for w in times.windows(2) {
            let (a, b) = (version_number(w[0]), version_number(w[1]));
            assert!(a < b && parts(&a) < parts(&b), "{a} < {b}");
        }
        // MMDD <= 1231 and HHMM <= 2359: every component fits 16 bits
        for s in times {
            assert!(parts(&version_number(s)).iter().all(|p| *p < 65_535));
        }
    }

    #[test]
    fn the_four_forms_and_unknown() {
        assert_eq!(
            version_string(T, git(true, false)),
            "2026.1028.1132 (7bd8876)"
        );
        assert_eq!(
            version_string(T, git(false, true)),
            "2026.1028.1132 (dev-7bd8876-dirty)"
        );
        assert_eq!(
            version_string(T, git(true, true)),
            "2026.1028.1132 (7bd8876-dirty)"
        );
        assert_eq!(
            version_string(T, git(false, false)),
            "2026.1028.1132 (dev-7bd8876)"
        );
        assert_eq!(version_string(T, None), "2026.1028.1132 (unknown)");
    }

    /// `scripts/version.sh` (the file names, the macOS bundle's CFBundleVersion, the tag) and the
    /// About box's number come from one SOURCE_DATE_EPOCH.
    #[cfg(unix)] // Windows: release.yml runs version.sh in Git Bash; `bash` there may be WSL
    #[test]
    fn version_sh_agrees_with_the_about_box() {
        use std::path::Path;
        use std::process::Command;
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/version.sh");
        let run = |epoch: i64, args: &[&str]| {
            let out = Command::new("bash")
                .arg(&script)
                .args(args)
                .env("SOURCE_DATE_EPOCH", epoch.to_string())
                .output()
                .expect("bash");
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            String::from_utf8(out.stdout).unwrap().trim().to_string()
        };
        for t in [
            0,
            T,
            1_793_215_800,
            1_709_164_799,
            1_767_225_599,
            4_107_542_400,
        ] {
            let n = version_number(t);
            assert_eq!(run(t, &[]), n, "epoch {t}");
            // the release tag v<version> maps back to the first second of that minute
            assert_eq!(
                run(0, &["--epoch", &format!("v{n}")]),
                (t - t % 60).to_string()
            );
        }
        assert_eq!(run(T, &["--time"]), "2026-10-28 11:32:11");
        for bad in [
            "v-2026_10_28_10_10_22",
            "v2026.1328.1930",
            "2026.1028.2560",
            "2026.1028.1930",
            "v2026.1028",
        ] {
            let out = Command::new("bash")
                .arg(&script)
                .args(["--epoch", bad])
                .output()
                .unwrap();
            assert!(!out.status.success(), "{bad}");
        }
    }

    #[test]
    fn the_branch_override_wins_over_git_and_github() {
        // no override: git's branch decides
        assert!(is_main(None, Some("main"), None));
        assert!(!is_main(None, Some("feature"), Some("main")));
        // a detached HEAD: GITHUB_REF_NAME decides (a tag is not main)
        assert!(is_main(None, Some("HEAD"), Some("main")));
        assert!(!is_main(None, Some("HEAD"), Some("v1.0")));
        assert!(!is_main(None, Some("HEAD"), None));
        assert!(is_main(None, None, Some("main")));
        assert!(!is_main(None, None, None));
        // ORB_BUILD_BRANCH: a release on a tag's detached HEAD is main
        assert!(is_main(Some("main"), Some("HEAD"), Some("v1.0")));
        assert!(is_main(Some("main"), Some("feature"), None));
        assert!(!is_main(Some("topic"), Some("main"), Some("main")));
        // an empty override is not set
        assert!(is_main(Some(""), Some("main"), None));
        assert!(is_main(Some(" main\n"), None, None));
    }
}
