//! The API contract is a committed file. Any route or schema change must show
//! up in review as a diff to docs/openapi.snapshot.json.
//!
//! Regenerate deliberately with: UPDATE_OPENAPI=1 cargo test -p trace-api --test openapi_snapshot

use std::path::Path;

fn path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/openapi.snapshot.json")
}

fn current() -> String {
    serde_json::to_string_pretty(&trace_api::routes::openapi::document()).unwrap() + "\n"
}

#[test]
fn the_openapi_document_matches_the_committed_snapshot() {
    let now = current();
    if std::env::var("UPDATE_OPENAPI").is_ok() {
        std::fs::write(path(), &now).unwrap();
        return;
    }
    let committed = std::fs::read_to_string(path())
        .expect("docs/openapi.snapshot.json is missing; run with UPDATE_OPENAPI=1");
    assert_eq!(
        committed, now,
        "API contract changed. If intended, regenerate with UPDATE_OPENAPI=1 and commit the diff."
    );
}

#[test]
fn every_operation_declares_error_and_auth_behaviour() {
    let doc = trace_api::routes::openapi::document();
    for (path, ops) in doc["paths"].as_object().unwrap() {
        for (method, op) in ops.as_object().unwrap() {
            assert!(
                op["responses"]["400"].is_object(),
                "{method} {path}: no 400"
            );
            if op["security"].is_array() {
                assert!(
                    op["responses"]["401"].is_object(),
                    "{method} {path}: secured but no 401"
                );
            }
        }
    }
}
