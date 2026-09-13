// SPDX-License-Identifier: Apache-2.0
// DB2 schema importer and DEL unload adapter.
//
// Reads DB2 catalog metadata (columns with name, db2_type, precision, scale,
// nullable, key) into a column schema, then turns DB2 DEL (character
// delimited) unload files into canonical CSV with fixed-point fidelity:
// DECIMAL(p,s) becomes an i64 scaled by 10^s with the scale recorded, never a
// float. Failure modes are loud: a row or field that does not match the
// declared schema fails the import naming the row number and field.

use std::fs::File;
use std::io::Write;
use std::path::Path;

/// A single DB2 column from a catalog export.
#[derive(Debug, Clone, PartialEq)]
pub struct Db2Column {
    pub name: String,
    pub db2_type: String,
    pub precision: u32,
    pub scale: u32,
    pub nullable: bool,
    pub key: bool,
}

/// True when the (upper-cased) db2 type is one this adapter understands.
fn is_known_type(t: &str) -> bool {
    matches!(
        t,
        "DECIMAL"
            | "CHAR"
            | "VARCHAR"
            | "DATE"
            | "TIME"
            | "TIMESTAMP"
            | "SMALLINT"
            | "INTEGER"
            | "BIGINT"
            | "DOUBLE"
            | "REAL"
    )
}

/// True when the db2 type can hold a negative value (drives the signed flag).
fn is_signed(t: &str) -> bool {
    matches!(t, "DECIMAL" | "SMALLINT" | "INTEGER" | "BIGINT" | "DOUBLE" | "REAL")
}

/// Parse a DB2 schema JSON (an array of column objects) into columns.
///
/// Rejects DECIMAL columns whose integer digits exceed i64 capacity
/// (precision - scale > 18) and unknown db2 types, naming the column in the
/// error.
pub fn parse_schema(json: &str) -> Result<Vec<Db2Column>, String> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("schema is not valid JSON: {e}"))?;
    let arr = value
        .as_array()
        .ok_or_else(|| "schema must be a JSON array of columns".to_string())?;
    let mut cols = Vec::with_capacity(arr.len());
    for item in arr {
        let obj = item
            .as_object()
            .ok_or_else(|| "schema columns must be JSON objects".to_string())?;
        let name = obj
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "schema column is missing a string 'name'".to_string())?
            .to_string();
        let db2_type = obj
            .get("db2_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("column {name}: missing string 'db2_type'"))?
            .to_ascii_uppercase();
        let precision = obj
            .get("precision")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let scale = obj.get("scale").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let nullable = obj.get("nullable").and_then(|v| v.as_bool()).unwrap_or(false);
        let key = obj.get("key").and_then(|v| v.as_bool()).unwrap_or(false);

        if !is_known_type(&db2_type) {
            return Err(format!("column {name}: unknown db2 type '{db2_type}'"));
        }
        if db2_type == "DECIMAL" && precision.saturating_sub(scale) > 18 {
            return Err(format!(
                "column {name}: DECIMAL({precision},{scale}) integer digits {} exceed i64 capacity (18)",
                precision - scale
            ));
        }
        cols.push(Db2Column {
            name,
            db2_type,
            precision,
            scale,
            nullable,
            key,
        });
    }
    Ok(cols)
}

/// Split one DEL line into its fields, honouring double-quote and comma
/// delimiters and doubled-quote escapes.
fn split_del_line(line: &str) -> Result<Vec<String>, String> {
    let b = line.as_bytes();
    let n = b.len();
    let mut fields: Vec<String> = Vec::new();
    let mut i = 0usize;
    while i < n {
        // DEL left-pads numeric fields with spaces; drop leading whitespace.
        while i < n && b[i] == b' ' {
            i += 1;
        }
        if i >= n {
            break;
        }
        let field_no = fields.len() + 1;
        if b[i] == b',' {
            fields.push(String::new());
            i += 1;
            continue;
        }
        if b[i] == b'"' {
            i += 1;
            let mut val = String::new();
            loop {
                if i >= n {
                    return Err(format!("unterminated quoted field at column {field_no}"));
                }
                if b[i] == b'"' {
                    if i + 1 < n && b[i + 1] == b'"' {
                        val.push('"');
                        i += 2;
                    } else {
                        i += 1;
                        break;
                    }
                } else {
                    let ch = line[i..].chars().next().unwrap();
                    val.push(ch);
                    i += ch.len_utf8();
                }
            }
            while i < n && b[i] == b' ' {
                i += 1;
            }
            if i < n {
                if b[i] == b',' {
                    i += 1;
                } else {
                    return Err(format!("expected ',' after quoted field at column {field_no}"));
                }
            }
            fields.push(val);
        } else {
            let start = i;
            while i < n && b[i] != b',' {
                i += 1;
            }
            let val = line[start..i].trim().to_string();
            if i < n {
                i += 1;
            }
            fields.push(val);
        }
    }
    if n > 0 && b[n - 1] == b',' {
        fields.push(String::new());
    }
    Ok(fields)
}

/// True when a DEL first line is the optional header (column names).
fn is_header(fields: &[String], schema: &[Db2Column]) -> bool {
    fields.len() == schema.len()
        && fields
            .iter()
            .zip(schema.iter())
            .all(|(f, c)| f.trim().eq_ignore_ascii_case(c.name.trim()))
}

/// Convert one DEL field to its canonical CSV representation.
fn convert_field(col: &Db2Column, raw: &str, row: usize) -> Result<String, String> {
    // '?' and the empty field are both NULL in a DEL unload.
    if raw == "?" || raw.is_empty() {
        if col.nullable {
            return Ok(String::new());
        }
        return Err(format!(
            "row {row}: field {}: NULL in non-nullable column",
            col.name
        ));
    }
    match col.db2_type.as_str() {
        "DECIMAL" => {
            let scaled = parse_decimal_scaled(raw, col.scale)
                .map_err(|e| format!("row {row}: field {}: {e}", col.name))?;
            Ok(scaled.to_string())
        }
        _ => Ok(raw.to_string()),
    }
}

/// Parse a DECIMAL unload string ('1234.56', '-1234.56', '+1234.56') into an
/// i64 scaled by 10^scale.
fn parse_decimal_scaled(raw: &str, scale: u32) -> Result<i64, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("empty DECIMAL value".to_string());
    }
    let (neg, body) = match s.as_bytes()[0] {
        b'-' => (true, &s[1..]),
        b'+' => (false, &s[1..]),
        _ => (false, s),
    };
    let (int_part, frac_part) = match body.split_once('.') {
        Some((i, f)) => (i, f),
        None => (body, ""),
    };
    if int_part.is_empty() && frac_part.is_empty() {
        return Err(format!("invalid DECIMAL value '{raw}'"));
    }
    if !int_part.chars().all(|c| c.is_ascii_digit())
        || !frac_part.chars().all(|c| c.is_ascii_digit())
    {
        return Err(format!("invalid DECIMAL value '{raw}'"));
    }
    if frac_part.len() as u32 > scale {
        return Err(format!(
            "DECIMAL value '{raw}' has more than {scale} fractional digits"
        ));
    }
    let mut digits = String::with_capacity(int_part.len() + scale as usize + 1);
    digits.push_str(int_part);
    digits.push_str(frac_part);
    for _ in (frac_part.len() as u32)..scale {
        digits.push('0');
    }
    let magnitude: i64 = digits
        .parse()
        .map_err(|_| format!("DECIMAL value '{raw}' overflows i64"))?;
    if neg {
        magnitude
            .checked_neg()
            .ok_or_else(|| format!("DECIMAL value '{raw}' overflows i64"))
    } else {
        Ok(magnitude)
    }
}

/// Quote a CSV field when it carries a delimiter, quote, or newline.
fn csv_quote(field: &str) -> String {
    // Comma, quote, line feed (10) and carriage return (13).
    let needs_quote = field
        .bytes()
        .any(|b| b == b',' || b == b'"' || b == 10 || b == 13);
    if needs_quote {
        let escaped = field.replace('"', r#""""#);
        let mut out = String::with_capacity(escaped.len() + 2);
        out.push('"');
        out.push_str(&escaped);
        out.push('"');
        out
    } else {
        field.to_string()
    }
}

/// Import a DB2 DEL unload into canonical CSV, returning the number of data
/// rows written. The header is taken from the schema; an optional DEL header
/// line (first row matching the column names) is skipped. DECIMAL fields are
/// scaled to i64; NULL ('?' or empty on a nullable column) becomes empty.
pub fn import_del(schema: &[Db2Column], del: &str, out_csv: &Path) -> Result<u64, String> {
    if schema.is_empty() {
        return Err("schema is empty: no columns to import".to_string());
    }
    let mut out =
        File::create(out_csv).map_err(|e| format!("cannot create {}: {e}", out_csv.display()))?;
    let header = schema
        .iter()
        .map(|c| csv_quote(&c.name))
        .collect::<Vec<_>>()
        .join(",");
    writeln!(out, "{header}").map_err(|e| format!("cannot write {}: {e}", out_csv.display()))?;

    let mut rows: u64 = 0;
    let mut first_line = true;
    let mut line_no = 0usize;
    for line in del.lines() {
        line_no += 1;
        if line.trim().is_empty() {
            continue;
        }
        let fields = split_del_line(line).map_err(|e| format!("row {line_no}: {e}"))?;
        if first_line {
            first_line = false;
            if is_header(&fields, schema) {
                continue;
            }
        }
        if fields.len() != schema.len() {
            return Err(format!(
                "row {line_no}: expected {} fields, found {}",
                schema.len(),
                fields.len()
            ));
        }
        let mut out_fields = Vec::with_capacity(schema.len());
        for (idx, col) in schema.iter().enumerate() {
            out_fields.push(convert_field(col, &fields[idx], line_no)?);
        }
        let out_line = out_fields
            .iter()
            .map(|f| csv_quote(f))
            .collect::<Vec<_>>()
            .join(",");
        writeln!(out, "{out_line}")
            .map_err(|e| format!("cannot write {}: {e}", out_csv.display()))?;
        rows += 1;
    }
    Ok(rows)
}

fn column_to_json(col: &Db2Column) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    obj.insert("name".to_string(), serde_json::Value::String(col.name.clone()));
    obj.insert(
        "kind".to_string(),
        serde_json::Value::String(col.db2_type.to_ascii_lowercase()),
    );
    obj.insert("length".to_string(), serde_json::Value::from(col.precision));
    obj.insert("scale".to_string(), serde_json::Value::from(col.scale));
    obj.insert("signed".to_string(), serde_json::Value::from(is_signed(&col.db2_type)));
    obj.insert("offset".to_string(), serde_json::Value::from(0u32));
    obj.insert("fixed_point".to_string(), serde_json::Value::from(col.scale > 0));
    obj.insert("children".to_string(), serde_json::Value::Array(Vec::new()));
    serde_json::Value::Object(obj)
}

/// Render columns as the canonical field-schema JSON shared with the copybook
/// path: an array of objects with name, kind, length, scale, signed, offset,
/// fixed_point, children.
pub fn schema_json(cols: &[Db2Column]) -> serde_json::Value {
    serde_json::Value::Array(cols.iter().map(column_to_json).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn decimal_col(name: &str, precision: u32, scale: u32, nullable: bool) -> Db2Column {
        Db2Column {
            name: name.to_string(),
            db2_type: "DECIMAL".to_string(),
            precision,
            scale,
            nullable,
            key: false,
        }
    }

    fn char_col(name: &str, precision: u32, nullable: bool) -> Db2Column {
        Db2Column {
            name: name.to_string(),
            db2_type: "CHAR".to_string(),
            precision,
            scale: 0,
            nullable,
            key: false,
        }
    }

    fn temp_csv(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("bylazora_db2_{}_{}.csv", std::process::id(), name))
    }

    fn read_output(path: &Path) -> String {
        std::fs::read_to_string(path).expect("read output csv")
    }

    fn remove(path: &Path) {
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn decimal_value_maps_to_scaled_integer_with_scale_in_schema() {
        let schema = vec![decimal_col("AMOUNT", 9, 2, false)];
        let out = temp_csv("decimal_scaled");
        let rows = import_del(&schema, "\"1234.56\"", &out).expect("import");
        assert_eq!(rows, 1);
        assert_eq!(read_output(&out), "AMOUNT\n123456\n");

        let json = schema_json(&schema);
        assert_eq!(json[0]["scale"].as_u64(), Some(2));
        remove(&out);
    }

    #[test]
    fn decimal_negative_and_explicitly_signed_values_round_trip() {
        let schema = vec![decimal_col("AMOUNT", 9, 2, false)];
        let out = temp_csv("decimal_signed");
        let del = "-1234.56\n+1234.56\n1234.56\n";
        let rows = import_del(&schema, del, &out).expect("import");
        assert_eq!(rows, 3);
        assert_eq!(read_output(&out), "AMOUNT\n-123456\n123456\n123456\n");
        remove(&out);
    }

    #[test]
    fn char_field_with_embedded_comma_is_preserved() {
        let schema = vec![char_col("ID", 9, false), char_col("NAME", 30, true)];
        let out = temp_csv("char_comma");
        let del = "\"42\",\"Doe, John\"\n";
        let rows = import_del(&schema, del, &out).expect("import");
        assert_eq!(rows, 1);
        assert_eq!(read_output(&out), "ID,NAME\n42,\"Doe, John\"\n");
        remove(&out);
    }

    #[test]
    fn null_marker_maps_to_empty_for_nullable_column() {
        let schema = vec![decimal_col("AMOUNT", 9, 2, true), char_col("NOTE", 20, true)];
        let out = temp_csv("null_marker");
        let del = "\"?\",\"?\"\n";
        let rows = import_del(&schema, del, &out).expect("import");
        assert_eq!(rows, 1);
        assert_eq!(read_output(&out), "AMOUNT,NOTE\n,\n");
        remove(&out);
    }

    #[test]
    fn del_header_line_is_skipped() {
        let schema = vec![decimal_col("AMOUNT", 9, 2, false), char_col("NAME", 20, false)];
        let out = temp_csv("header_skip");
        let del = "AMOUNT,NAME\n\"1.50\",\"Alice\"\n\"2.00\",\"Bob\"\n";
        let rows = import_del(&schema, del, &out).expect("import");
        assert_eq!(rows, 2);
        assert_eq!(read_output(&out), "AMOUNT,NAME\n150,Alice\n200,Bob\n");
        remove(&out);
    }

    #[test]
    fn schema_json_round_trips_canonical_shape() {
        let json = r#"[
            {"name":"AMOUNT","db2_type":"DECIMAL","precision":9,"scale":2,"nullable":false,"key":true},
            {"name":"NAME","db2_type":"VARCHAR","precision":30,"scale":0,"nullable":true,"key":false}
        ]"#;
        let cols = parse_schema(json).expect("valid schema");
        assert_eq!(cols.len(), 2);
        assert_eq!(cols[0].name, "AMOUNT");
        assert_eq!(cols[0].scale, 2);

        let out = schema_json(&cols);
        let arr = out.as_array().expect("schema is an array");
        assert_eq!(arr.len(), 2);

        let amount = &arr[0];
        assert_eq!(amount["name"].as_str(), Some("AMOUNT"));
        assert_eq!(amount["kind"].as_str(), Some("decimal"));
        assert_eq!(amount["length"].as_u64(), Some(9));
        assert_eq!(amount["scale"].as_u64(), Some(2));
        assert_eq!(amount["signed"].as_bool(), Some(true));
        assert_eq!(amount["fixed_point"].as_bool(), Some(true));
        assert_eq!(amount["offset"].as_u64(), Some(0));
        assert!(amount["children"].as_array().is_some());

        let name = &arr[1];
        assert_eq!(name["name"].as_str(), Some("NAME"));
        assert_eq!(name["kind"].as_str(), Some("varchar"));
        assert_eq!(name["signed"].as_bool(), Some(false));
        assert_eq!(name["fixed_point"].as_bool(), Some(false));
    }

    #[test]
    fn bad_row_fails_naming_row_and_field() {
        let schema = vec![decimal_col("AMOUNT", 9, 2, false), char_col("NAME", 20, false)];
        let out = temp_csv("bad_row");
        let del = "\"1.50\",\"Alice\"\n\"abc\",\"Bob\"\n";
        let err = import_del(&schema, del, &out).unwrap_err();
        assert!(err.contains("row 2"), "got: {err}");
        assert!(err.contains("AMOUNT"), "got: {err}");
        remove(&out);
    }

    #[test]
    fn decimal_precision_beyond_i64_is_rejected_at_schema_parse() {
        let json = r#"[{"name":"BIG","db2_type":"DECIMAL","precision":20,"scale":0,"nullable":false,"key":false}]"#;
        let err = parse_schema(json).unwrap_err();
        assert!(err.contains("BIG"), "got: {err}");
        assert!(err.contains("18"), "got: {err}");
    }

    #[test]
    fn unknown_db2_type_is_rejected_with_column_name() {
        let json = r#"[{"name":"OBJ","db2_type":"BLOB2","precision":0,"scale":0,"nullable":true,"key":false}]"#;
        let err = parse_schema(json).unwrap_err();
        assert!(err.contains("OBJ"), "got: {err}");
        assert!(err.contains("BLOB2"), "got: {err}");
    }

    #[test]
    fn null_marker_in_non_nullable_column_fails() {
        let schema = vec![decimal_col("AMOUNT", 9, 2, false)];
        let out = temp_csv("null_non_nullable");
        let err = import_del(&schema, "\"?\"\n", &out).unwrap_err();
        assert!(err.contains("row 1"), "got: {err}");
        assert!(err.contains("AMOUNT"), "got: {err}");
        remove(&out);
    }

    #[test]
    fn empty_field_on_nullable_column_becomes_empty() {
        let schema = vec![char_col("NOTE", 20, true)];
        let out = temp_csv("empty_nullable");
        let rows = import_del(&schema, "\"\"\n", &out).expect("import");
        assert_eq!(rows, 1);
        assert_eq!(read_output(&out), "NOTE\n\n");
        remove(&out);
    }
}
