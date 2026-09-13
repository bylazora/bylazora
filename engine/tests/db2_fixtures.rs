// SPDX-License-Identifier: AGPL-3.0-or-later
// Integration test for the P2 DB2 importer using the worked example that
// ships as the schema format's only documentation (README.md and the site
// point readers at these two files, so they need a caller that actually
// exercises them, not just sit in the crate root unused).
use bylazora::db2;
use std::path::Path;

fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn smoke_schema_covers_bigint_varchar_decimal_and_date() {
    let schema_json = fixture("smoke-schema.json");
    let cols = db2::parse_schema(&schema_json).expect("smoke-schema.json should parse");
    let types: Vec<&str> = cols.iter().map(|c| c.db2_type.as_str()).collect();
    assert_eq!(types, ["BIGINT", "VARCHAR", "DECIMAL", "DATE"]);
}

#[test]
fn smoke_unload_imports_three_rows_with_scaled_decimals_and_embedded_comma() {
    let schema_json = fixture("smoke-schema.json");
    let cols = db2::parse_schema(&schema_json).expect("smoke-schema.json should parse");
    let del_text = fixture("smoke-unload.del");
    let out = std::env::temp_dir().join(format!("bylazora-db2-fixture-{}.csv", std::process::id()));
    let rows = db2::import_del(&cols, &del_text, &out).expect("smoke-unload.del should import");
    assert_eq!(rows, 3);
    let csv = std::fs::read_to_string(&out).unwrap();
    let _ = std::fs::remove_file(&out);
    assert!(csv.contains("\"Doe, Jane\""), "embedded comma should survive quoted: {csv}");
    assert!(csv.contains("123456"), "1234.56 should scale to integer cents: {csv}");
    assert!(csv.contains("-9990"), "-99.90 should scale to integer cents: {csv}");
    assert!(csv.contains("250000"), "+2500.00 should scale to integer cents: {csv}");
}
