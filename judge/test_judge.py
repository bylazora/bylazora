"""Tests for the gate-as-judge harness (lib.judge).

The judge compares a candidate backend's output directory against the COBOL
reference through lib.validate.compare_outputs and issues a single verdict.
"""
import json
import pathlib
import subprocess
import sys

import pytest

from lib.judge import judge

FINAL = "ACCOUNT_ID,BALANCE_CENTS\n0,1600\n1,2800\n"
SUMMARY = ("ACCOUNT_ID,TOTAL_DEPOSITS,TOTAL_WITHDRAWALS,TXN_COUNT,ENDING_BALANCE\n"
           "0,800,200,3,1600\n-1,3750,1550,10,17200\n")

REPO_ROOT = pathlib.Path(__file__).resolve().parents[1]


def write_dir(base: pathlib.Path, final: str = FINAL, summary: str = SUMMARY) -> pathlib.Path:
    base.mkdir(parents=True, exist_ok=True)
    (base / "final_balances.csv").write_text(final)
    (base / "summary_report.csv").write_text(summary)
    return base


@pytest.fixture
def ref(tmp_path):
    return write_dir(tmp_path / "ref")


def test_identical_dirs_proven(ref, tmp_path):
    result = judge(ref, write_dir(tmp_path / "candidate"))
    assert result["verdict"] == "proven"
    assert result["files_compared"] == 2
    assert result["mismatch_count"] == 0
    assert result["mismatches"] == []


def test_one_byte_diff_failed_named(ref, tmp_path):
    candidate = write_dir(tmp_path / "candidate", final=FINAL.replace("1600", "1601"))
    result = judge(ref, candidate)
    assert result["verdict"] == "failed"
    assert result["mismatch_count"] == 1
    assert "final_balances.csv" in result["mismatches"][0]


def test_missing_candidate_file_failed(ref, tmp_path):
    candidate = write_dir(tmp_path / "candidate")
    (candidate / "final_balances.csv").unlink()
    result = judge(ref, candidate)
    assert result["verdict"] == "failed"
    assert "final_balances.csv" in result["mismatches"][0]


def test_missing_reference_file_not_proven(ref, tmp_path):
    (ref / "final_balances.csv").unlink()
    result = judge(ref, write_dir(tmp_path / "candidate"))
    assert result["verdict"] == "failed"
    assert "final_balances.csv" in result["mismatches"][0]
    assert "reference missing" in result["mismatches"][0]


def test_empty_files_list_proven_only_if_identical(ref, tmp_path):
    identical = write_dir(tmp_path / "identical")
    differing = write_dir(tmp_path / "differing", final=FINAL.replace("2800", "2801"))
    assert judge(ref, identical, files=[])["verdict"] == "proven"
    assert judge(ref, differing, files=[])["verdict"] == "failed"


def test_judge_identity_field(ref, tmp_path):
    assert judge(ref, write_dir(tmp_path / "candidate"))["judge"] == "bylazora-gate"


def run_judge(ref_dir, candidate_dir, *extra):
    return subprocess.run(
        [sys.executable, "-m", "lib.judge",
         "--ref-dir", str(ref_dir), "--candidate-dir", str(candidate_dir), *extra],
        cwd=REPO_ROOT, capture_output=True, text=True,
    )


def test_cli_proven_exits_zero(ref, tmp_path):
    r = run_judge(ref, write_dir(tmp_path / "candidate"))
    assert r.returncode == 0, r.stderr
    assert json.loads(r.stdout)["verdict"] == "proven"


def test_cli_failed_exits_one(ref, tmp_path):
    r = run_judge(ref, write_dir(tmp_path / "candidate", final=FINAL.replace("1600", "1601")))
    assert r.returncode == 1, r.stderr
    assert json.loads(r.stdout)["verdict"] == "failed"


def test_cli_files_comma_option(ref, tmp_path):
    candidate = write_dir(tmp_path / "candidate", summary=SUMMARY.replace("800", "801"))
    r = run_judge(ref, candidate, "--files", "final_balances.csv")
    assert r.returncode == 0, r.stderr
    assert json.loads(r.stdout)["verdict"] == "proven"
