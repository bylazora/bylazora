// SPDX-License-Identifier: Apache-2.0
// COBOL copybook and record-layout parser.
//
// Parses column-free COBOL copybooks into record-layout schemas so migrated
// estates can be ingested byte-exactly. Produces decoders for EBCDIC, fixed,
// and binary layouts with fixed-point fidelity (money as integer cents).

use std::collections::HashMap;

/// A single field in a record layout. Groups hold their children in order.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub level: u32,
    pub pic: String,
    pub kind: FieldKind,
    pub length: usize,
    pub scale: u32,
    pub signed: bool,
    pub occurs: u32,
    pub redefines: Option<String>,
    pub children: Vec<Field>,
    pub offset: usize,
}

/// How a field is stored and therefore decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Display,
    Comp,
    Comp3,
    Binary,
    Group,
    Filler,
}

impl FieldKind {
    fn as_str(self) -> &'static str {
        match self {
            FieldKind::Display => "display",
            FieldKind::Comp => "comp",
            FieldKind::Comp3 => "comp3",
            FieldKind::Binary => "binary",
            FieldKind::Group => "group",
            FieldKind::Filler => "filler",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Usage {
    Comp,
    Comp3,
    Binary,
}

struct PicInfo {
    alpha: bool,
    digits: usize,
    scale: u32,
    signed: bool,
}

struct RawField {
    level: u32,
    name: String,
    pic: String,
    kind: FieldKind,
    length: usize,
    scale: u32,
    signed: bool,
    occurs: u32,
    redefines: Option<String>,
    line: usize,
}

/// Strip a trailing inline comment, or the whole line when it is a comment.
fn strip_comment(line: &str) -> String {
    if line.trim_start().starts_with('*') {
        return String::new();
    }
    match line.find(" *>") {
        Some(i) => line[..i].to_string(),
        None => line.to_string(),
    }
}

/// Collect non-blank, comment-free lines with their 1-based line numbers.
fn logical_lines(src: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (idx, raw) in src.lines().enumerate() {
        let stripped = strip_comment(raw);
        let trimmed = stripped.trim();
        if !trimmed.is_empty() {
            out.push((idx + 1, trimmed.to_string()));
        }
    }
    out
}

/// True when a line begins a new field: a level number (01..49) then a space.
fn is_level_line(line: &str) -> bool {
    let b = line.as_bytes();
    let mut i = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    i > 0 && i <= 2 && i < b.len() && b[i] == b' '
}

/// Group physical lines into field statements: a level line plus its trailing
/// continuation lines (for example a PIC clause on the following line).
fn group_statements(lines: Vec<(usize, String)>) -> Result<Vec<(usize, String)>, String> {
    let mut grouped: Vec<(usize, Vec<String>)> = Vec::new();
    for (no, line) in lines {
        if is_level_line(&line) {
            grouped.push((no, vec![line]));
        } else if let Some(last) = grouped.last_mut() {
            last.1.push(line);
        } else {
            return Err(format!("line {no}: expected a level number, found '{line}'"));
        }
    }
    Ok(grouped
        .into_iter()
        .map(|(no, parts)| (no, parts.join(" ")))
        .collect())
}

fn parse_x_count(s: &str) -> Option<usize> {
    if s.is_empty() {
        return Some(1);
    }
    if s.starts_with('(') && s.ends_with(')') {
        return s[1..s.len() - 1].parse::<usize>().ok();
    }
    if s.chars().all(|c| c == 'X') {
        return Some(s.len());
    }
    None
}

fn parse_nine_count(s: &str) -> Option<usize> {
    if s == "9" {
        return Some(1);
    }
    if s.starts_with("9(") && s.ends_with(')') {
        return s[2..s.len() - 1].parse::<usize>().ok();
    }
    if !s.is_empty() && s.chars().all(|c| c == '9') {
        return Some(s.len());
    }
    None
}

/// Parse a PIC string into its shape. Editing pictures are out of scope.
fn parse_pic(raw: &str) -> Result<PicInfo, String> {
    let p = raw.trim().to_ascii_uppercase();
    if p.is_empty() {
        return Err("empty PIC".to_string());
    }
    if p.contains('Z')
        || p.contains('$')
        || p.contains('*')
        || p.contains('+')
        || p.contains('-')
        || p.contains(',')
        || p.contains('.')
        || p.contains('/')
        || p.contains('B')
    {
        return Err(format!("editing PIC not supported: {raw}"));
    }

    if let Some(rest) = p.strip_prefix('X') {
        let n = parse_x_count(rest).ok_or_else(|| format!("unsupported PIC: {raw}"))?;
        return Ok(PicInfo { alpha: true, digits: n, scale: 0, signed: false });
    }

    let (signed, body) = match p.strip_prefix('S') {
        Some(b) => (true, b),
        None => (false, p.as_str()),
    };
    if body.is_empty() || !body.starts_with('9') {
        return Err(format!("unsupported PIC: {raw}"));
    }
    let mut parts = body.split('V');
    let int_part = parts.next().unwrap();
    let frac_part = parts.next();
    if parts.next().is_some() {
        return Err(format!("unsupported PIC: {raw}"));
    }
    let digits = parse_nine_count(int_part).ok_or_else(|| format!("unsupported PIC: {raw}"))?;
    let scale = match frac_part {
        None => 0,
        Some(f) => parse_nine_count(f).ok_or_else(|| format!("unsupported PIC: {raw}"))? as u32,
    };
    Ok(PicInfo { alpha: false, digits, scale, signed })
}

fn comp_length(digits: usize) -> Result<usize, String> {
    match digits {
        1..=4 => Ok(2),
        5..=9 => Ok(4),
        10..=18 => Ok(8),
        _ => Err(format!("COMP/BINARY PIC 9({digits}) exceeds 18 digits")),
    }
}

fn parse_statement(statement: &str, line: usize) -> Result<RawField, String> {
    let tokens: Vec<String> = statement
        .split_whitespace()
        .map(|t| t.trim_end_matches('.').to_string())
        .collect();

    let first = tokens.first().ok_or_else(|| format!("line {line}: empty field"))?;
    let level: u32 = first
        .as_str()
        .parse()
        .map_err(|_| format!("line {line}: invalid level '{first}'"))?;
    if !(1..=49).contains(&level) {
        return Err(format!("line {line}: level {level} out of range (01..49)"));
    }
    let name = tokens
        .get(1)
        .ok_or_else(|| format!("line {line}: field at level {level} is missing a name"))?
        .clone();

    let mut pic: Option<String> = None;
    let mut usage: Option<Usage> = None;
    let mut occurs: u32 = 1;
    let mut redefines: Option<String> = None;

    let mut i = 2;
    while i < tokens.len() {
        match tokens[i].to_ascii_uppercase().as_str() {
            "PIC" | "PICTURE" => {
                if pic.is_some() {
                    return Err(format!("line {line}: duplicate PIC clause"));
                }
                let p = tokens
                    .get(i + 1)
                    .ok_or_else(|| format!("line {line}: PIC clause is missing its picture string"))?;
                pic = Some(p.clone());
                i += 2;
            }
            "COMP-3" => {
                if usage.is_some() {
                    return Err(format!("line {line}: duplicate usage clause"));
                }
                usage = Some(Usage::Comp3);
                i += 1;
            }
            "COMP-5" | "BINARY" => {
                if usage.is_some() {
                    return Err(format!("line {line}: duplicate usage clause"));
                }
                usage = Some(Usage::Binary);
                i += 1;
            }
            "COMP" => {
                if usage.is_some() {
                    return Err(format!("line {line}: duplicate usage clause"));
                }
                usage = Some(Usage::Comp);
                i += 1;
            }
            "OCCURS" => {
                if occurs != 1 {
                    return Err(format!("line {line}: duplicate OCCURS clause"));
                }
                let next = tokens
                    .get(i + 1)
                    .ok_or_else(|| format!("line {line}: OCCURS clause is missing its count"))?;
                if next.to_ascii_uppercase() == "DEPENDING" {
                    return Err(format!("line {line}: OCCURS DEPENDING ON is not supported"));
                }
                let n: u32 = next
                    .as_str()
                    .parse()
                    .map_err(|_| format!("line {line}: OCCURS count '{next}' is not a number"))?;
                if n == 0 {
                    return Err(format!("line {line}: OCCURS count must be at least 1"));
                }
                occurs = n;
                i += 2;
                if i < tokens.len() && tokens[i].to_ascii_uppercase() == "TIMES" {
                    i += 1;
                }
            }
            "REDEFINES" => {
                if redefines.is_some() {
                    return Err(format!("line {line}: duplicate REDEFINES clause"));
                }
                let target = tokens
                    .get(i + 1)
                    .ok_or_else(|| format!("line {line}: REDEFINES clause is missing its target field"))?;
                redefines = Some(target.clone());
                i += 2;
            }
            "TIMES" => {
                return Err(format!("line {line}: unexpected TIMES clause"));
            }
            other => {
                return Err(format!("line {line}: unsupported clause '{other}'"));
            }
        }
    }

    let (kind, length, scale, signed, pic_string) = match &pic {
        None => {
            if usage.is_some() {
                return Err(format!("line {line}: a group field cannot have a usage clause"));
            }
            (FieldKind::Group, 0usize, 0u32, false, String::new())
        }
        Some(p) => {
            let pi = parse_pic(p).map_err(|e| format!("line {line}: {e}"))?;
            if name == "FILLER" {
                if usage.is_some() {
                    return Err(format!("line {line}: FILLER does not support COMP usage"));
                }
                let len = if pi.alpha { pi.digits } else { pi.digits + pi.scale as usize };
                (FieldKind::Filler, len, pi.scale, pi.signed, p.clone())
            } else {
                match usage {
                    Some(Usage::Comp3) => {
                        if pi.alpha {
                            return Err(format!("line {line}: COMP-3 requires a numeric PIC"));
                        }
                        let len = (pi.digits + pi.scale as usize) / 2 + 1;
                        (FieldKind::Comp3, len, pi.scale, pi.signed, p.clone())
                    }
                    Some(Usage::Comp) => {
                        if pi.alpha {
                            return Err(format!("line {line}: COMP requires a numeric PIC"));
                        }
                        let len = comp_length(pi.digits).map_err(|e| format!("line {line}: {e}"))?;
                        (FieldKind::Comp, len, pi.scale, pi.signed, p.clone())
                    }
                    Some(Usage::Binary) => {
                        if pi.alpha {
                            return Err(format!("line {line}: BINARY/COMP-5 requires a numeric PIC"));
                        }
                        let len = comp_length(pi.digits).map_err(|e| format!("line {line}: {e}"))?;
                        (FieldKind::Binary, len, pi.scale, pi.signed, p.clone())
                    }
                    None => {
                        let len = if pi.alpha { pi.digits } else { pi.digits + pi.scale as usize };
                        (FieldKind::Display, len, pi.scale, pi.signed, p.clone())
                    }
                }
            }
        }
    };

    Ok(RawField {
        level,
        name,
        pic: pic_string,
        kind,
        length,
        scale,
        signed,
        occurs,
        redefines,
        line,
    })
}

struct Frame {
    level: u32,
    group: Field,
    children: Vec<Field>,
    running: usize,
    names: HashMap<String, usize>,
}

fn empty_group() -> Field {
    Field {
        name: String::new(),
        level: 0,
        pic: String::new(),
        kind: FieldKind::Group,
        length: 0,
        scale: 0,
        signed: false,
        occurs: 1,
        redefines: None,
        children: Vec::new(),
        offset: 0,
    }
}

fn attach_closed(frames: &mut Vec<Frame>, mut closed: Frame) {
    let is_redefines = closed.group.redefines.is_some();
    closed.group.children = closed.children;
    closed.group.length = closed.running;
    if !is_redefines {
        frames.last_mut().unwrap().running += closed.group.length;
    }
    frames.last_mut().unwrap().children.push(closed.group);
}

fn build(raws: Vec<RawField>) -> Result<Vec<Field>, String> {
    let mut frames: Vec<Frame> = vec![Frame {
        level: 0,
        group: empty_group(),
        children: Vec::new(),
        running: 0,
        names: HashMap::new(),
    }];

    for raw in raws {
        while frames.len() > 1 && frames.last().unwrap().level >= raw.level {
            let closed = frames.pop().unwrap();
            attach_closed(&mut frames, closed);
        }

        let field = Field {
            name: raw.name,
            level: raw.level,
            pic: raw.pic,
            kind: raw.kind,
            length: raw.length,
            scale: raw.scale,
            signed: raw.signed,
            occurs: raw.occurs,
            redefines: raw.redefines,
            children: Vec::new(),
            offset: 0,
        };
        let line = raw.line;

        let offset = match &field.redefines {
            Some(target) => *frames
                .last()
                .unwrap()
                .names
                .get(target)
                .ok_or_else(|| {
                    format!("line {line}: REDEFINES target '{target}' is not declared at the same level")
                })?,
            None => frames.last().unwrap().running,
        };
        let is_redefines = field.redefines.is_some();
        frames.last_mut().unwrap().names.insert(field.name.clone(), offset);

        if field.kind == FieldKind::Group {
            let mut group = field;
            group.offset = offset;
            frames.push(Frame {
                level: raw.level,
                group,
                children: Vec::new(),
                running: 0,
                names: HashMap::new(),
            });
        } else {
            let mut leaf = field;
            leaf.offset = offset;
            if !is_redefines {
                frames.last_mut().unwrap().running += leaf.length * leaf.occurs as usize;
            }
            frames.last_mut().unwrap().children.push(leaf);
        }
    }

    while frames.len() > 1 {
        let closed = frames.pop().unwrap();
        attach_closed(&mut frames, closed);
    }
    Ok(frames.pop().unwrap().children)
}

/// Parse a copybook into its top-level 01 records with offsets.
pub fn parse_copybook(src: &str) -> Result<Vec<Field>, String> {
    let lines = logical_lines(src);
    let statements = group_statements(lines)?;
    let mut raws = Vec::with_capacity(statements.len());
    for (line, statement) in statements {
        raws.push(parse_statement(&statement, line)?);
    }
    build(raws)
}

fn field_to_json(field: &Field) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    obj.insert("name".to_string(), serde_json::Value::String(field.name.clone()));
    obj.insert("level".to_string(), serde_json::Value::from(field.level));
    obj.insert("kind".to_string(), serde_json::Value::String(field.kind.as_str().to_string()));
    obj.insert("length".to_string(), serde_json::Value::from(field.length));
    obj.insert("scale".to_string(), serde_json::Value::from(field.scale));
    obj.insert("signed".to_string(), serde_json::Value::from(field.signed));
    obj.insert("occurs".to_string(), serde_json::Value::from(field.occurs));
    obj.insert("offset".to_string(), serde_json::Value::from(field.offset));
    obj.insert("fixed_point".to_string(), serde_json::Value::from(field.scale > 0));
    obj.insert(
        "children".to_string(),
        serde_json::Value::Array(field.children.iter().map(field_to_json).collect()),
    );
    serde_json::Value::Object(obj)
}

/// Render fields as a JobSpec-style JSON schema. Money fields (a fractional
/// scale) are flagged fixed-point.
pub fn field_schema(fields: &[Field]) -> serde_json::Value {
    serde_json::Value::Array(fields.iter().map(field_to_json).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find<'a>(fields: &'a [Field], name: &str) -> &'a Field {
        fields
            .iter()
            .find(|f| f.name.as_str() == name)
            .unwrap_or_else(|| panic!("field {name} not found"))
    }

    const CUSTOMER: &str = r#"01 CUSTOMER-REC.
   05 CUSTOMER-ID PIC 9(9).
   05 CUSTOMER-NAME PIC X(30).
   05 CUSTOMER-BALANCE PIC S9(9)V99 COMP-3.
   05 CUSTOMER-TYPE PIC X.
   05 FILLER PIC X(6)."#;

    #[test]
    fn customer_record_parses_with_kinds_and_offsets() {
        let fields = parse_copybook(CUSTOMER).expect("valid copybook");
        assert_eq!(fields.len(), 1);
        let rec = &fields[0];
        assert_eq!(rec.name, "CUSTOMER-REC");
        assert_eq!(rec.level, 1);
        assert_eq!(rec.kind, FieldKind::Group);
        assert_eq!(rec.length, 52);
        assert_eq!(rec.offset, 0);
        assert_eq!(rec.children.len(), 5);

        let id = find(&rec.children, "CUSTOMER-ID");
        assert_eq!(id.kind, FieldKind::Display);
        assert_eq!(id.length, 9);
        assert_eq!(id.scale, 0);
        assert!(!id.signed);
        assert_eq!(id.offset, 0);
        assert_eq!(id.occurs, 1);

        let name = find(&rec.children, "CUSTOMER-NAME");
        assert_eq!(name.kind, FieldKind::Display);
        assert_eq!(name.length, 30);
        assert_eq!(name.offset, 9);

        let balance = find(&rec.children, "CUSTOMER-BALANCE");
        assert_eq!(balance.kind, FieldKind::Comp3);
        assert_eq!(balance.length, 6);
        assert_eq!(balance.scale, 2);
        assert!(balance.signed);
        assert_eq!(balance.offset, 39);

        let typ = find(&rec.children, "CUSTOMER-TYPE");
        assert_eq!(typ.kind, FieldKind::Display);
        assert_eq!(typ.length, 1);
        assert_eq!(typ.offset, 45);

        let filler = find(&rec.children, "FILLER");
        assert_eq!(filler.kind, FieldKind::Filler);
        assert_eq!(filler.length, 6);
        assert_eq!(filler.offset, 46);
    }

    #[test]
    fn nested_groups_build_tree_and_group_offsets() {
        let src = r#"01 EMPLOYEE-REC.
   05 EMP-ID PIC 9(5).
   05 EMP-NAME.
      10 FIRST-NAME PIC X(15).
      10 LAST-NAME PIC X(20).
   05 EMP-SALARY PIC 9(7)V99 COMP-3."#;
        let fields = parse_copybook(src).expect("valid copybook");
        let rec = &fields[0];
        assert_eq!(rec.length, 45);

        let emp_name = find(&rec.children, "EMP-NAME");
        assert_eq!(emp_name.kind, FieldKind::Group);
        assert_eq!(emp_name.level, 5);
        assert_eq!(emp_name.offset, 5);
        assert_eq!(emp_name.length, 35);
        assert_eq!(emp_name.children.len(), 2);
        assert_eq!(emp_name.children[0].name, "FIRST-NAME");
        assert_eq!(emp_name.children[0].level, 10);
        assert_eq!(emp_name.children[0].offset, 0);
        assert_eq!(emp_name.children[1].name, "LAST-NAME");
        assert_eq!(emp_name.children[1].offset, 15);

        let salary = find(&rec.children, "EMP-SALARY");
        assert_eq!(salary.offset, 40);
        assert_eq!(salary.length, 5);
        assert_eq!(salary.scale, 2);
    }

    #[test]
    fn redefines_shares_target_offset() {
        let src = r#"01 CUST-REC.
   05 CUST-ID PIC X(10).
   05 CUST-ALT REDEFINES CUST-ID PIC 9(10).
   05 CUST-NAME PIC X(20)."#;
        let fields = parse_copybook(src).expect("valid copybook");
        let rec = &fields[0];
        assert_eq!(rec.length, 30);

        let alt = find(&rec.children, "CUST-ALT");
        assert_eq!(alt.redefines.as_deref(), Some("CUST-ID"));
        assert_eq!(alt.offset, 0);

        let name = find(&rec.children, "CUST-NAME");
        assert_eq!(name.offset, 10);
    }

    #[test]
    fn occurs_fixed_count_advances_offset() {
        let src = r#"01 TABLE-REC.
   05 ENTRY-COUNT PIC 9(3).
   05 ENTRIES OCCURS 12 PIC X(4).
   05 CHECKSUM PIC 9(4)."#;
        let fields = parse_copybook(src).expect("valid copybook");
        let rec = &fields[0];

        let entries = find(&rec.children, "ENTRIES");
        assert_eq!(entries.occurs, 12);
        assert_eq!(entries.length, 4);
        assert_eq!(entries.offset, 3);

        let checksum = find(&rec.children, "CHECKSUM");
        assert_eq!(checksum.offset, 3 + 12 * 4);
    }

    #[test]
    fn comp_and_binary_sizing_boundaries() {
        let src = r#"01 NUMBERS.
   05 N4 PIC 9(4) COMP.
   05 N9 PIC 9(9) COMP.
   05 N18 PIC 9(18) COMP.
   05 B9 PIC 9(9) BINARY.
   05 C5 PIC 9(9) COMP-5."#;
        let fields = parse_copybook(src).expect("valid copybook");
        let rec = &fields[0];

        let n4 = find(&rec.children, "N4");
        assert_eq!(n4.kind, FieldKind::Comp);
        assert_eq!(n4.length, 2);
        assert_eq!(n4.offset, 0);

        let n9 = find(&rec.children, "N9");
        assert_eq!(n9.kind, FieldKind::Comp);
        assert_eq!(n9.length, 4);
        assert_eq!(n9.offset, 2);

        let n18 = find(&rec.children, "N18");
        assert_eq!(n18.kind, FieldKind::Comp);
        assert_eq!(n18.length, 8);
        assert_eq!(n18.offset, 6);

        let b9 = find(&rec.children, "B9");
        assert_eq!(b9.kind, FieldKind::Binary);
        assert_eq!(b9.length, 4);
        assert_eq!(b9.offset, 14);

        let c5 = find(&rec.children, "C5");
        assert_eq!(c5.kind, FieldKind::Binary);
        assert_eq!(c5.length, 4);
        assert_eq!(c5.offset, 18);
    }

    #[test]
    fn display_signed_with_v_has_scale() {
        let src = r#"01 MONEY.
   05 AMT PIC S9(9)V99.
   05 RATE PIC 9(3)V9(4)."#;
        let fields = parse_copybook(src).expect("valid copybook");
        let rec = &fields[0];

        let amt = find(&rec.children, "AMT");
        assert_eq!(amt.kind, FieldKind::Display);
        assert_eq!(amt.length, 11);
        assert_eq!(amt.scale, 2);
        assert!(amt.signed);

        let rate = find(&rec.children, "RATE");
        assert_eq!(rate.length, 7);
        assert_eq!(rate.scale, 4);
        assert!(!rate.signed);
    }

    #[test]
    fn field_schema_flags_money_as_fixed_point() {
        let fields = parse_copybook(CUSTOMER).expect("valid copybook");
        let schema = field_schema(&fields);
        let arr = schema.as_array().expect("schema is an array");
        assert_eq!(arr.len(), 1);

        let rec = &arr[0];
        assert_eq!(rec["name"].as_str(), Some("CUSTOMER-REC"));
        assert_eq!(rec["kind"].as_str(), Some("group"));
        assert_eq!(rec["length"].as_u64(), Some(52));

        let children = rec["children"].as_array().expect("children array");
        let balance = children
            .iter()
            .find(|c| c["name"].as_str() == Some("CUSTOMER-BALANCE"))
            .expect("balance present");
        assert_eq!(balance["kind"].as_str(), Some("comp3"));
        assert_eq!(balance["length"].as_u64(), Some(6));
        assert_eq!(balance["scale"].as_u64(), Some(2));
        assert_eq!(balance["signed"].as_bool(), Some(true));
        assert_eq!(balance["fixed_point"].as_bool(), Some(true));

        let id = children
            .iter()
            .find(|c| c["name"].as_str() == Some("CUSTOMER-ID"))
            .expect("id present");
        assert_eq!(id["fixed_point"].as_bool(), Some(false));
        assert_eq!(id["offset"].as_u64(), Some(0));
    }

    #[test]
    fn editing_pic_is_rejected() {
        let src = r#"01 R.
   05 A PIC Z(9)."#;
        let err = parse_copybook(src).unwrap_err();
        assert!(err.contains("editing PIC not supported"), "got: {err}");
        assert!(err.contains("Z(9)"), "got: {err}");
    }

    #[test]
    fn occurs_depending_on_is_rejected() {
        let src = r#"01 R.
   05 A OCCURS DEPENDING ON N PIC X(1)."#;
        let err = parse_copybook(src).unwrap_err();
        assert!(err.contains("DEPENDING"), "got: {err}");
    }

    #[test]
    fn unknown_level_is_rejected() {
        let src = r#"50 BAD-LEVEL PIC X(1)."#;
        let err = parse_copybook(src).unwrap_err();
        assert!(err.contains("level"), "got: {err}");
        assert!(err.contains("out of range"), "got: {err}");
    }

    #[test]
    fn errors_name_the_line() {
        let src = r#"01 R.
   05 A PIC Z(9)."#;
        let err = parse_copybook(src).unwrap_err();
        assert!(err.contains("line 2"), "got: {err}");
    }

    #[test]
    fn pic_on_following_line_is_accepted() {
        let src = r#"01 R.
   05 A
      PIC X(8)."#;
        let fields = parse_copybook(src).expect("valid copybook");
        let a = find(&fields[0].children, "A");
        assert_eq!(a.kind, FieldKind::Display);
        assert_eq!(a.length, 8);
    }

    #[test]
    fn inline_and_whole_line_comments_are_stripped() {
        let src = r#"* header comment
01 R. *> trailing comment
   05 A PIC X(4). *> inline
   * full-line comment
   05 B PIC 9(2)."#;
        let fields = parse_copybook(src).expect("valid copybook");
        let rec = &fields[0];
        assert_eq!(rec.children.len(), 2);
        assert_eq!(find(&rec.children, "A").length, 4);
        assert_eq!(find(&rec.children, "B").offset, 4);
    }
}
