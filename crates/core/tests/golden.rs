//! Golden test: the deterministic `analysis.json` for the committed fixture
//! must match the checked-in snapshot (ignoring the volatile timestamp).
//!
//! Regenerate the snapshot after an intentional output change with:
//!   See `docs/RELEASE_CHECKLIST.md`.

use codeatlas_core::pipeline::{analyze, AnalyzeOptions};
use serde_json::Value;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/sample_repo");
const GOLDEN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/sample_repo.analysis.golden.json"
);

#[test]
fn sample_repo_matches_golden_analysis() {
    let result = analyze(&AnalyzeOptions {
        repo_path: FIXTURE.into(),
        project_name: "sample_repo".into(),
        depth_cap: 3,
    })
    .expect("fixture analysis succeeds");

    let mut generated = serde_json::to_value(&result.analysis).unwrap();
    let golden: Value = serde_json::from_str(&std::fs::read_to_string(GOLDEN).unwrap()).unwrap();

    // `generated_at` is the only non-deterministic field.
    generated["metadata"]["generated_at"] = Value::String("<normalized>".to_string());

    assert_eq!(generated, golden);
}
