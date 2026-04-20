use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use jiff::Timestamp;

// =============================================================================
// Build metadata capture
// =============================================================================
//
// `strz version` should report which revision produced the current binary without
// requiring Git to be available at runtime. We therefore capture the short commit
// SHA and a UTC build date here and expose them through compile-time env vars.
fn main() {
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
    println!("cargo:rerun-if-env-changed=STRZ_BUILD_GIT_SHA");

    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("Cargo should set CARGO_MANIFEST_DIR"));
    register_git_inputs(&manifest_dir);

    println!(
        "cargo:rustc-env=STRZ_BUILD_GIT_SHA={}",
        build_git_sha(&manifest_dir)
    );
    println!("cargo:rustc-env=STRZ_BUILD_DATE={}", build_date());
}

fn build_git_sha(manifest_dir: &Path) -> String {
    resolve_git_sha(
        env::var("STRZ_BUILD_GIT_SHA").ok().as_deref(),
        git_rev_parse(manifest_dir),
    )
}

fn resolve_git_sha(override_sha: Option<&str>, repo_sha: Option<String>) -> String {
    override_sha.map_or_else(
        || repo_sha.unwrap_or_else(|| "unknown".to_owned()),
        |sha| normalize_git_sha(sha).unwrap_or_else(|| "unknown".to_owned()),
    )
}

fn normalize_git_sha(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.chars().take(7).collect())
}

fn git_rev_parse(manifest_dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .current_dir(manifest_dir)
        .args(["rev-parse", "--short=7", "HEAD"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    normalize_git_sha(&stdout)
}

fn build_date() -> String {
    env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|seconds| seconds.trim().parse::<i64>().ok())
        .and_then(|seconds| Timestamp::from_second(seconds).ok())
        .map_or_else(|| format_build_date(Timestamp::now()), format_build_date)
}

fn format_build_date(timestamp: Timestamp) -> String {
    let rendered = timestamp.to_string();
    let (date, _) = rendered
        .split_once('T')
        .expect("timestamp display should include a date/time separator");
    date.to_owned()
}

fn register_git_inputs(manifest_dir: &Path) {
    let Some(repo_root) = manifest_dir.ancestors().nth(2) else {
        return;
    };
    let Some(git_dir) = resolve_git_dir(&repo_root.join(".git")) else {
        return;
    };

    for path in git_rerun_paths(&git_dir) {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

fn git_rerun_paths(git_dir: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let head_path = git_dir.join("HEAD");
    if head_path.exists() {
        paths.push(head_path.clone());
    }

    let packed_refs_path = git_dir.join("packed-refs");
    if packed_refs_path.exists() {
        paths.push(packed_refs_path);
    }

    if let Some(head_ref) = git_head_ref(&head_path) {
        let ref_path = git_dir.join(head_ref);
        if ref_path.exists() {
            paths.push(ref_path);
        }
    }

    paths
}

fn git_head_ref(head_path: &Path) -> Option<PathBuf> {
    let contents = fs::read_to_string(head_path).ok()?;
    let reference = contents.strip_prefix("ref: ")?.trim();
    Some(PathBuf::from(reference))
}

fn resolve_git_dir(dot_git_path: &Path) -> Option<PathBuf> {
    if dot_git_path.is_dir() {
        return Some(dot_git_path.to_path_buf());
    }

    let contents = fs::read_to_string(dot_git_path).ok()?;
    let git_dir = contents.strip_prefix("gitdir: ")?.trim();
    Some(dot_git_path.parent()?.join(git_dir))
}

#[cfg(test)]
mod tests {
    use jiff::Timestamp;

    use super::{format_build_date, normalize_git_sha, resolve_git_sha};

    #[test]
    fn normalize_git_sha_trims_and_truncates() {
        assert_eq!(
            normalize_git_sha(" abc123456 \n").as_deref(),
            Some("abc1234")
        );
    }

    #[test]
    fn resolve_git_sha_prefers_an_explicit_blank_override() {
        assert_eq!(
            resolve_git_sha(Some("  "), Some("abcdef1".to_owned())),
            "unknown"
        );
    }

    #[test]
    fn resolve_git_sha_falls_back_to_repo_sha_when_no_override_is_set() {
        assert_eq!(resolve_git_sha(None, Some("abcdef1".to_owned())), "abcdef1");
    }

    #[test]
    fn format_build_date_returns_the_utc_calendar_date() {
        let timestamp = Timestamp::from_second(1_735_689_600).expect("timestamp should be valid");
        assert_eq!(format_build_date(timestamp), "2025-01-01");
    }
}
