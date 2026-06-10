//! Shared conformance suite: assert the Rust engine reproduces the committed
//! golden outputs in `conformance/expected/`. Every implementation runs the same
//! cases against the same goldens (see conformance/README.md), so all stay in
//! step. Regenerate goldens with `cargo run --bin conformance -- --generate`.

use chrono::NaiveDate;
use greenbook::{evaluate, load_product_map, load_schedule, parse_fhir_bundle};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize)]
struct Case {
    id: String,
    fixture: String,
    schedule: String,
    products: String,
    evaluated_at: NaiveDate,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn matches_conformance_goldens() {
    let root = repo_root();
    let cases: Vec<Case> =
        serde_json::from_str(&fs::read_to_string(root.join("conformance/cases.json")).unwrap())
            .unwrap();

    for case in &cases {
        let schedule = load_schedule(&root.join(&case.schedule)).unwrap();
        let products = load_product_map(&root.join(&case.products)).unwrap();
        let bundle =
            fs::read_to_string(root.join("conformance/fixtures").join(&case.fixture)).unwrap();
        let record = parse_fhir_bundle(&bundle).unwrap();
        let status = evaluate(&record, &schedule, &products, case.evaluated_at).unwrap();

        let got = serde_json::to_value(&status).unwrap();
        let golden = root
            .join("conformance/expected")
            .join(format!("{}.json", case.id));
        let want: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&golden).expect("golden file exists"))
                .unwrap();

        assert_eq!(got, want, "conformance mismatch for case '{}'", case.id);
    }
}
