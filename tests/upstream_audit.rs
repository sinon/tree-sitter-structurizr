mod common;

use std::collections::BTreeMap;
use std::env;

use serde::Deserialize;

const DEFAULT_UNSUPPORTED_FILTERS: &[&str] = &["script", "plugin"];
const ALWAYS_IGNORED_FILTERS: &[&str] = &["unexpected-", "multi-line-with-error"];

#[derive(Debug, Deserialize)]
struct GitHubContent {
    r#type: String,
    name: String,
    path: String,
    download_url: Option<String>,
}

#[derive(Debug)]
struct FileFailure {
    path: String,
    issues: Vec<common::ParseIssue>,
}

#[test]
#[ignore = "Downloads upstream Structurizr DSL samples and validates them against the local parser"]
fn upstream_structurizr_samples_parse_without_errors() {
    let client = reqwest::blocking::Client::builder()
        .user_agent("tree-sitter-structurizr-upstream-audit")
        .build()
        .expect("failed to build HTTP client");

    let mut entries: Vec<GitHubContent> = client
        .get("https://api.github.com/repos/structurizr/structurizr/contents/structurizr-dsl/src/test/resources/dsl")
        .send()
        .expect("failed to fetch upstream DSL listing")
        .error_for_status()
        .expect("upstream DSL listing request failed")
        .json()
        .expect("failed to decode upstream DSL listing");

    entries.retain(|entry| entry.r#type == "file" && entry.name.ends_with(".dsl"));
    entries.sort_by(|a, b| a.path.cmp(&b.path));

    if let Ok(filter) = env::var("STRUCTURIZR_UPSTREAM_FILTER") {
        entries.retain(|entry| entry.path.contains(&filter));
    }

    let before_always_ignored = entries.len();
    entries.retain(|entry| !is_always_ignored(&entry.path));
    let excluded_as_negative_cases = before_always_ignored - entries.len();

    let include_unsupported = env_flag("STRUCTURIZR_UPSTREAM_INCLUDE_UNSUPPORTED");
    let excluded_by_default = if include_unsupported {
        0usize
    } else {
        let before = entries.len();
        entries.retain(|entry| !is_explicitly_unsupported(&entry.path));
        before - entries.len()
    };

    let mut clean = 0usize;
    let mut failures = Vec::new();
    let mut breakdown = BTreeMap::<String, usize>::new();

    for entry in entries {
        let source = client
            .get(entry.download_url.as_deref().expect("missing download_url"))
            .send()
            .unwrap_or_else(|error| panic!("failed to download `{}`: {error}", entry.path))
            .error_for_status()
            .unwrap_or_else(|error| panic!("download failed for `{}`: {error}", entry.path))
            .text()
            .unwrap_or_else(|error| panic!("failed to read `{}`: {error}", entry.path));

        let tree = common::parse(&source);
        let issues = common::collect_parse_issues(&tree, &source);

        if issues.is_empty() {
            clean += 1;
        } else {
            *breakdown.entry(feature_bucket(&entry.path).to_string()).or_default() += 1;
            failures.push(FileFailure {
                path: entry.path,
                issues,
            });
        }
    }

    println!(
        "Checked {} upstream DSL files: {} clean, {} failing",
        clean + failures.len(),
        clean,
        failures.len()
    );

    if excluded_as_negative_cases > 0 {
        println!(
            "Ignored {} upstream DSL files permanently as intentional negative tests ({})",
            excluded_as_negative_cases,
            ALWAYS_IGNORED_FILTERS
                .iter()
                .map(|pattern| format!("contains `{pattern}`"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    if excluded_by_default > 0 {
        println!(
            "Excluded {} upstream DSL files by default as explicitly unsupported ({})",
            excluded_by_default,
            DEFAULT_UNSUPPORTED_FILTERS
                .iter()
                .map(|pattern| format!("contains `{pattern}`"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    if !breakdown.is_empty() {
        println!("\nBreakdown by feature area:");
        for (feature, count) in &breakdown {
            println!("- {feature}: {count}");
        }
    }

    if !failures.is_empty() {
        println!("\nFailing files and extracted issue text:");
        for failure in &failures {
            println!("\n- {}", failure.path);
            for issue in failure.issues.iter().take(5) {
                println!(
                    "  - {} {} [{}:{}-{}:{}] `{}`",
                    issue.kind,
                    issue.node_kind,
                    issue.start.row + 1,
                    issue.start.column + 1,
                    issue.end.row + 1,
                    issue.end.column + 1,
                    issue.text
                );
            }
            if failure.issues.len() > 5 {
                println!("  - ... {} more issue nodes", failure.issues.len() - 5);
            }
        }
    }

    assert!(
        failures.is_empty(),
        "upstream audit found {} failing DSL files",
        failures.len()
    );
}

fn feature_bucket(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.contains("archetype") || lower.contains("custom-element") || lower.contains("find-element") {
        "archetypes and custom elements"
    } else if lower.contains("deployment") || lower.contains("amazon-web-services") {
        "deployment"
    } else if lower.contains("dynamic") || lower.contains("parallel") || lower.contains("animation") {
        "dynamic views"
    } else if lower.contains("include") || lower.contains("workspace-extension") {
        "workspace extension and include"
    } else if lower.contains("script") || lower.contains("plugin") {
        "scripts and plugins"
    } else if lower.contains("group") {
        "groups"
    } else if lower.contains("relationship") || lower.contains("filtered") || lower.contains("exclude") {
        "relationships and expressions"
    } else if lower.contains("style") || lower.contains("theme") || lower.contains("color") || lower.contains("shape") {
        "styles and themes"
    } else if lower.contains("identifier") || lower.contains("constant") {
        "identifiers and constants"
    } else if lower.contains("text-block") || lower.contains("multi-line") {
        "text blocks"
    } else {
        "other"
    }
}

fn is_explicitly_unsupported(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    DEFAULT_UNSUPPORTED_FILTERS
        .iter()
        .any(|pattern| lower.contains(pattern))
}

fn is_always_ignored(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    ALWAYS_IGNORED_FILTERS
        .iter()
        .any(|pattern| lower.contains(pattern))
}

fn env_flag(name: &str) -> bool {
    env::var(name)
        .map(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}
