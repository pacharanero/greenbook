//! Conformance golden generator (Rust is the reference implementation).
//!
//!   cargo run --bin conformance -- --generate   # (re)write conformance/expected/
//!   cargo run --bin conformance                 # check current output matches goldens
//!
//! Reads `conformance/cases.json`, evaluates each case against the canonical
//! schedule + product map + fixture, and writes/checks the golden output JSON
//! in `conformance/expected/`. Every implementation's test suite then asserts
//! it reproduces these goldens (see conformance/README.md), so all stay in step.

use chrono::NaiveDate;
use greenbook::{evaluate, load_product_map, load_schedule, parse_fhir_bundle};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Deserialize)]
struct Case {
    id: String,
    fixture: String,
    schedule: String,
    products: String,
    evaluated_at: NaiveDate,
}

/// Repository root: the crate lives in `rust/`, so go one level up.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn main() -> ExitCode {
    let generate = std::env::args().any(|a| a == "--generate");
    let root = repo_root();

    let cases_json =
        fs::read_to_string(root.join("conformance/cases.json")).expect("read cases.json");
    let cases: Vec<Case> = serde_json::from_str(&cases_json).expect("parse cases.json");

    let out_dir = root.join("conformance/expected");
    fs::create_dir_all(&out_dir).expect("create expected dir");

    let mut mismatches = 0;
    for case in &cases {
        let schedule = load_schedule(&root.join(&case.schedule)).expect("load schedule");
        let products = load_product_map(&root.join(&case.products)).expect("load products");
        let bundle = fs::read_to_string(root.join("conformance/fixtures").join(&case.fixture))
            .expect("read fixture");
        let record = parse_fhir_bundle(&bundle).expect("parse fixture");
        let status = evaluate(&record, &schedule, &products, case.evaluated_at).expect("evaluate");

        let json = serde_json::to_string_pretty(&status).expect("serialize");
        let golden = out_dir.join(format!("{}.json", case.id));

        if generate {
            fs::write(&golden, format!("{json}\n")).expect("write golden");
            println!("wrote {}", golden.display());
        } else {
            let expected = fs::read_to_string(&golden).unwrap_or_default();
            let got: serde_json::Value = serde_json::from_str(&json).unwrap();
            let want: serde_json::Value =
                serde_json::from_str(&expected).unwrap_or(serde_json::Value::Null);
            if got == want {
                println!("ok   {}", case.id);
            } else {
                mismatches += 1;
                println!("FAIL {} (run with --generate to update goldens)", case.id);
            }
        }
    }

    if mismatches > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
