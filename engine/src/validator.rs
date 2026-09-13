// SPDX-License-Identifier: AGPL-3.0-or-later
// Byte-exact validator: the only component that marks a run proven.
use anyhow::Result;
use std::fs;
use std::path::Path;

pub const OUTPUT_FILES: [&str; 2] = ["final_balances.csv", "summary_report.csv"];

pub fn compare_outputs(ref_dir: &Path, other_dir: &Path) -> Result<Vec<String>> {
    let mut errs = Vec::new();
    for name in OUTPUT_FILES {
        let a = ref_dir.join(name);
        let b = other_dir.join(name);
        if !a.exists() {
            errs.push(format!("{name}: reference missing in {}", ref_dir.display()));
            continue;
        }
        if !b.exists() {
            errs.push(format!("{name}: missing in {}", other_dir.display()));
            continue;
        }
        if fs::read(&a)? != fs::read(&b)? {
            errs.push(format!("{name}: content differs"));
        }
    }
    Ok(errs)
}
