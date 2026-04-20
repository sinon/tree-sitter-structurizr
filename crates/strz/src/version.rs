use crate::report::VersionReport;

/// Returns the compile-time build metadata exposed by `strz version`.
#[must_use]
pub fn build_report() -> VersionReport {
    VersionReport {
        name: "strz".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        git_sha: env!("STRZ_BUILD_GIT_SHA").to_owned(),
        build_date: env!("STRZ_BUILD_DATE").to_owned(),
    }
}
