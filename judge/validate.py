"""Byte-exact comparison of backend output directories (COBOL = reference)."""
from __future__ import annotations

import difflib
import pathlib

OUTPUT_FILES = ("final_balances.csv", "summary_report.csv")


def compare_outputs(ref_dir: pathlib.Path, other_dir: pathlib.Path,
                      files: tuple[str, ...] | None = None) -> list[str]:
    if files is None:
        files = OUTPUT_FILES
    errors: list[str] = []
    for name in files:
        ref_file = pathlib.Path(ref_dir) / name
        other_file = pathlib.Path(other_dir) / name
        if not ref_file.exists():
            errors.append(f"{name}: reference missing in {ref_dir}")
            continue
        if not other_file.exists():
            errors.append(f"{name}: missing in {other_dir}")
            continue
        if ref_file.read_bytes() != other_file.read_bytes():
            diff = difflib.unified_diff(
                ref_file.read_text().splitlines(),
                other_file.read_text().splitlines(),
                fromfile=f"ref/{name}", tofile=f"other/{name}", lineterm="")
            errors.append(f"{name}: content differs\n" + "\n".join(diff))
    return errors
