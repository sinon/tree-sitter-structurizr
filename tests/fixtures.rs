mod common;

use std::path::PathBuf;

use rstest::rstest;

macro_rules! set_snapshot_suffix {
    ($($expr:expr),* $(,)?) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_snapshot_suffix(format!($($expr),*));
        let _guard = settings.bind_to_scope();
    };
}

#[rstest]
fn fixtures_match_expected_parse_outcomes(
    #[files("tests/fixtures/**/*.dsl")] path: PathBuf,
) {
    let fixture = common::load_fixture(&path);
    let tree = common::parse(&fixture.source);

    match fixture.expectation {
        common::FixtureExpectation::ParseOk => {
            common::assert_no_errors(&fixture.name, &tree, &fixture.source);
        }
        common::FixtureExpectation::ParseError => {
            common::assert_has_errors(&fixture.name, &tree, &fixture.source);
        }
    }

    set_snapshot_suffix!("{}", fixture.name);
    insta::assert_snapshot!("fixture", common::tree_sexp(&tree));
}
