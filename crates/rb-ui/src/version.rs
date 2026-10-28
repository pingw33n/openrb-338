//! The version string of the app (task-102): `<UTC build time> (<git>)`, for example
//! `2026-10-28 11:32:11 (7bd8876)`. This file has no dependencies: `build.rs` includes it
//! (`#[path]`) to compute the string, and the library tests it.
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
/// - `override_` (`RB_BUILD_BRANCH`, set by release builds): main or anything else;
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

/// `YYYY-MM-DD HH:MM:SS` of the Unix time `secs`, in UTC.
pub fn format_utc(secs: i64) -> String {
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
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// The version string: the build time and the git state, `(unknown)` with no git.
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
    format!("{} ({g})", format_utc(secs))
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
    fn the_time_is_utc() {
        assert_eq!(format_utc(0), "1970-01-01 00:00:00");
        assert_eq!(format_utc(T), "2026-10-28 11:32:11");
        // a leap day and the end of a year
        assert_eq!(format_utc(1_709_164_799), "2024-02-28 23:59:59");
        assert_eq!(format_utc(951_782_400), "2000-02-29 00:00:00");
        // 2100 is no leap year
        assert_eq!(format_utc(4_107_542_400), "2100-03-01 00:00:00");
        assert_eq!(format_utc(1_709_164_800), "2024-02-29 00:00:00");
        assert_eq!(format_utc(1_709_251_200), "2024-03-01 00:00:00");
        assert_eq!(format_utc(1_767_225_599), "2025-12-31 23:59:59");
    }

    #[test]
    fn the_four_forms_and_unknown() {
        assert_eq!(
            version_string(T, git(true, false)),
            "2026-10-28 11:32:11 (7bd8876)"
        );
        assert_eq!(
            version_string(T, git(false, true)),
            "2026-10-28 11:32:11 (dev-7bd8876-dirty)"
        );
        assert_eq!(
            version_string(T, git(true, true)),
            "2026-10-28 11:32:11 (7bd8876-dirty)"
        );
        assert_eq!(
            version_string(T, git(false, false)),
            "2026-10-28 11:32:11 (dev-7bd8876)"
        );
        assert_eq!(version_string(T, None), "2026-10-28 11:32:11 (unknown)");
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
        // RB_BUILD_BRANCH: a release on a tag's detached HEAD is main
        assert!(is_main(Some("main"), Some("HEAD"), Some("v1.0")));
        assert!(is_main(Some("main"), Some("feature"), None));
        assert!(!is_main(Some("topic"), Some("main"), Some("main")));
        // an empty override is not set
        assert!(is_main(Some(""), Some("main"), None));
        assert!(is_main(Some(" main\n"), None, None));
    }
}
