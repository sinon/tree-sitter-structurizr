#![allow(clippy::significant_drop_tightening)]

use std::path::{Path, PathBuf};

use criterion::{
    BatchSize, BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main,
};
use structurizr_analysis::{DocumentAnalyzer, DocumentInput, WorkspaceLoader};

const SMALL_DOCUMENT_SOURCE: &str =
    include_str!("../../structurizr-lsp/tests/fixtures/identifiers/direct-references-ok.dsl");
const MEDIUM_DOCUMENT_SOURCE: &str = include_str!(
    "../../../tests/lsp/workspaces/big-bank-plc/model/people-and-software-systems.dsl"
);
const LARGE_DOCUMENT_SOURCE: &str =
    include_str!("../../../tests/lsp/workspaces/big-bank-plc/internet-banking-system.dsl");
const MEGA_DOCUMENT_SOURCE: &str =
    include_str!("../../../tests/lsp/workspaces/benchmark-mega/workspace_data/ws-12/model/10-systems.dsl");

#[derive(Clone, Copy)]
struct DocumentCase {
    name: &'static str,
    id: &'static str,
    source: &'static str,
}

impl DocumentCase {
    fn input(self) -> DocumentInput {
        DocumentInput::new(self.id, self.source.to_owned())
    }
}

#[derive(Clone, Copy)]
struct WorkspaceCase {
    name: &'static str,
    relative_roots: &'static [&'static str],
    dsl_file_count: u64,
}

impl WorkspaceCase {
    fn roots(self) -> Vec<PathBuf> {
        self.relative_roots
            .iter()
            .map(|relative_root| {
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../..")
                    .join(relative_root)
            })
            .collect()
    }
}

const DOCUMENT_CASES: &[DocumentCase] = &[
    DocumentCase {
        name: "small_direct_references",
        id: "small_direct_references",
        source: SMALL_DOCUMENT_SOURCE,
    },
    DocumentCase {
        name: "medium_people_and_software_systems",
        id: "medium_people_and_software_systems",
        source: MEDIUM_DOCUMENT_SOURCE,
    },
    DocumentCase {
        name: "large_big_bank_workspace",
        id: "large_big_bank_workspace",
        source: LARGE_DOCUMENT_SOURCE,
    },
    DocumentCase {
        name: "mega_workspace_systems",
        id: "mega_workspace_systems",
        source: MEGA_DOCUMENT_SOURCE,
    },
];

const WORKSPACE_CASES: &[WorkspaceCase] = &[
    WorkspaceCase {
        name: "small_minimal_scan",
        relative_roots: &["tests/lsp/workspaces/minimal-scan"],
        dsl_file_count: 3,
    },
    WorkspaceCase {
        name: "medium_directory_include",
        relative_roots: &["tests/lsp/workspaces/directory-include"],
        dsl_file_count: 4,
    },
    WorkspaceCase {
        name: "large_big_bank_plc",
        relative_roots: &["tests/lsp/workspaces/big-bank-plc"],
        dsl_file_count: 6,
    },
    WorkspaceCase {
        name: "mega_benchmark_estate",
        relative_roots: &["tests/lsp/workspaces/benchmark-mega"],
        dsl_file_count: 252,
    },
    WorkspaceCase {
        name: "mega_benchmark_multi_root",
        relative_roots: &[
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-00/workspace.dsl",
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-01/workspace.dsl",
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-02/workspace.dsl",
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-03/workspace.dsl",
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-04/workspace.dsl",
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-05/workspace.dsl",
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-06/workspace.dsl",
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-07/workspace.dsl",
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-08/workspace.dsl",
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-09/workspace.dsl",
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-10/workspace.dsl",
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-11/workspace.dsl",
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-12/workspace.dsl",
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-13/workspace.dsl",
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-14/workspace.dsl",
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-15/workspace.dsl",
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-16/workspace.dsl",
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-17/workspace.dsl",
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-18/workspace.dsl",
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-19/workspace.dsl",
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-20/workspace.dsl",
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-21/workspace.dsl",
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-22/workspace.dsl",
            "tests/lsp/workspaces/benchmark-mega-multi-root/ws-23/workspace.dsl",
        ],
        dsl_file_count: 72,
    },
];

// The benchmark matrix keeps the original tiny and realistic cases, then adds a
// generated mega corpus to expose superlinear behavior in document extraction
// and workspace discovery before it reaches user-visible regressions.
fn bench_document_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("analysis/document");

    for case in DOCUMENT_CASES {
        let throughput =
            u64::try_from(case.source.len()).expect("document fixture size should fit into u64");
        group.throughput(Throughput::Bytes(throughput));
        group.bench_with_input(BenchmarkId::from_parameter(case.name), case, |b, case| {
            let mut analyzer = DocumentAnalyzer::new();

            b.iter_batched(
                || case.input(),
                |input| {
                    let snapshot = analyzer.analyze(input);
                    black_box(snapshot);
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn bench_workspace_loading(c: &mut Criterion) {
    let mut group = c.benchmark_group("analysis/workspace");

    for case in WORKSPACE_CASES {
        let roots = case.roots();
        group.throughput(Throughput::Elements(case.dsl_file_count));
        group.bench_with_input(BenchmarkId::from_parameter(case.name), case, |b, _case| {
            b.iter(|| {
                let mut loader = WorkspaceLoader::new();
                let facts = loader
                    .load_paths(roots.iter().map(PathBuf::as_path))
                    .expect("workspace benchmark fixture should load");
                black_box(facts);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_document_analysis, bench_workspace_loading);
criterion_main!(benches);
