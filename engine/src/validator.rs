// SPDX-License-Identifier: AGPL-3.0-or-later
// Byte-exact validator: the only component that marks a run proven.
use anyhow::Result;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// Used only when the reference directory has no files at all (an empty or
/// missing estate, most often a fresh clone's fixture directories), so a
/// developer running the gate cold still gets a meaningful list of what is
/// missing rather than a silently empty comparison.
pub const OUTPUT_FILES: [&str; 2] = ["final_balances.csv", "summary_report.csv"];

fn file_names(dir: &Path) -> BTreeSet<String> {
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter_map(|e| e.file_name().into_string().ok())
        .collect()
}

pub fn compare_outputs(ref_dir: &Path, other_dir: &Path) -> Result<Vec<String>> {
    let ref_files = file_names(ref_dir);
    let other_files = file_names(other_dir);
    // The reference declares what "the outputs" are; other_dir's own files
    // are folded in too so an extra file the candidate produced (that the
    // reference doesn't have) is a visible mismatch, not silently ignored.
    let mut names: BTreeSet<String> = ref_files.iter().chain(other_files.iter()).cloned().collect();
    if names.is_empty() {
        names = OUTPUT_FILES.iter().map(|s| s.to_string()).collect();
    }

    let mut errs = Vec::new();
    for name in names {
        let a = ref_dir.join(&name);
        let b = other_dir.join(&name);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("bylazora-validator-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn identical_two_file_dirs_report_no_mismatch() {
        let a = temp_dir("id-a");
        let b = temp_dir("id-b");
        fs::write(a.join("final_balances.csv"), "x").unwrap();
        fs::write(b.join("final_balances.csv"), "x").unwrap();
        fs::write(a.join("summary_report.csv"), "y").unwrap();
        fs::write(b.join("summary_report.csv"), "y").unwrap();
        assert!(compare_outputs(&a, &b).unwrap().is_empty());
    }

    #[test]
    fn one_byte_difference_is_a_mismatch() {
        let a = temp_dir("diff-a");
        let b = temp_dir("diff-b");
        fs::write(a.join("final_balances.csv"), "x").unwrap();
        fs::write(b.join("final_balances.csv"), "y").unwrap();
        let errs = compare_outputs(&a, &b).unwrap();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].contains("content differs"));
    }

    #[test]
    fn empty_reference_directory_falls_back_to_the_two_sample_names() {
        let a = temp_dir("empty-ref");
        let b = temp_dir("empty-other");
        let errs = compare_outputs(&a, &b).unwrap();
        assert_eq!(errs.len(), OUTPUT_FILES.len());
        for name in OUTPUT_FILES {
            assert!(errs.iter().any(|e| e.contains(name)), "missing {name} in {errs:?}");
        }
    }

    #[test]
    fn candidate_has_an_extra_file_the_reference_never_declared() {
        let a = temp_dir("extra-a");
        let b = temp_dir("extra-b");
        fs::write(a.join("final_balances.csv"), "x").unwrap();
        fs::write(b.join("final_balances.csv"), "x").unwrap();
        fs::write(b.join("audit_log.csv"), "unexpected").unwrap();
        let errs = compare_outputs(&a, &b).unwrap();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].contains("audit_log.csv"), "{errs:?}");
        assert!(errs[0].contains("reference missing"), "{errs:?}");
    }

    #[test]
    fn reference_has_a_third_file_the_candidate_got_wrong() {
        let a = temp_dir("third-a");
        let b = temp_dir("third-b");
        fs::write(a.join("final_balances.csv"), "x").unwrap();
        fs::write(b.join("final_balances.csv"), "x").unwrap();
        fs::write(a.join("reconciliation.csv"), "must match too").unwrap();
        let errs = compare_outputs(&a, &b).unwrap();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].contains("reconciliation.csv"), "{errs:?}");
        assert!(errs[0].contains("missing in"), "{errs:?}");
    }
}
