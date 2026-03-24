mod common;

#[test]
fn passing_fixtures_parse_without_errors() {
    for fixture in common::load_fixtures("tests/fixtures/pass") {
        let tree = common::parse(&fixture.source);

        common::assert_no_errors(&fixture.name, &tree, &fixture.source);
        insta::assert_snapshot!(fixture.snapshot_name(), common::tree_sexp(&tree));
    }
}

#[test]
fn future_structurizr_fixtures_are_tracked_as_pending_coverage() {
    for fixture in common::load_fixtures("tests/fixtures/future") {
        let tree = common::parse(&fixture.source);

        common::assert_has_errors(&fixture.name, &tree, &fixture.source);
        assert!(
            fixture.path.exists(),
            "fixture path should continue to exist: {}",
            fixture.path.display()
        );
    }
}
