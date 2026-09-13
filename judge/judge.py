# SPDX-License-Identifier: AGPL-3.0-or-later
"""Gate-as-judge: one verdict from a byte-exact comparison against the COBOL reference.

The judge compares a candidate backend's output directory against the legacy
COBOL reference through lib.validate.compare_outputs and issues a single
verdict. It never marks anything proven by any other means.
"""
from __future__ import annotations

import argparse
import json
import pathlib

from lib.validate import OUTPUT_FILES, compare_outputs

JUDGE_ID = "bylazora-gate"


def judge(ref_dir: pathlib.Path, candidate_dir: pathlib.Path,
          files: list[str] | None = None) -> dict:
    """Return the bylazora-gate verdict for a candidate output directory.

    The verdict is "proven" only when every declared output file is present and
    byte-identical in both directories. A missing candidate file, a missing
    reference file, or any byte difference is a failure. With no files declared,
    the judge compares every file the reference produced instead of proving a
    vacuous empty comparison.
    """
    ref_dir = pathlib.Path(ref_dir)
    candidate_dir = pathlib.Path(candidate_dir)

    if files is None:
        declared = list(OUTPUT_FILES)
    else:
        declared = list(files)

    if not declared:
        try:
            declared = sorted(p.name for p in ref_dir.iterdir() if p.is_file())
        except OSError:
            declared = []

    if not declared:
        return {
            "verdict": "failed",
            "files_compared": 0,
            "mismatch_count": 0,
            "mismatches": ["no output files to compare; equivalence is unproven"],
            "judge": JUDGE_ID,
        }

    mismatches = compare_outputs(ref_dir, candidate_dir, tuple(declared))
    return {
        "verdict": "proven" if not mismatches else "failed",
        "files_compared": len(declared),
        "mismatch_count": len(mismatches),
        "mismatches": mismatches,
        "judge": JUDGE_ID,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="python -m lib.judge",
        description="Score a candidate backend against the COBOL reference.",
    )
    parser.add_argument("--ref-dir", required=True,
                        help="directory holding the COBOL reference outputs")
    parser.add_argument("--candidate-dir", required=True,
                        help="directory holding the candidate outputs")
    parser.add_argument("--files", default=None,
                        help="comma-separated output files to compare "
                             "(default: the two engine outputs)")
    args = parser.parse_args(argv)

    files = None
    if args.files is not None:
        files = [name.strip() for name in args.files.split(",") if name.strip()]

    result = judge(pathlib.Path(args.ref_dir), pathlib.Path(args.candidate_dir), files)
    print(json.dumps(result, indent=2))
    return 0 if result["verdict"] == "proven" else 1


if __name__ == "__main__":
    raise SystemExit(main())
