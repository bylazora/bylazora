#!/usr/bin/env bash
set -u
cd ~
for s in 1m 10m 100m 1b; do
  echo "== $s =="
  for r in 1 2 3; do
    rm -rf /tmp/rep_$s
    ./core-rs/target/release/bylazora-core bench ~/mainframe-migration-poc/data/$s /tmp/rep_$s --backend gpu 2>&1 | grep -E "read\+dispatch|wrote outputs" | tr '\n' ' '
    ./core-rs/target/release/bylazora-core validate --ref-dir ~/mainframe-migration-poc/results/${s}-cobol --other-dir /tmp/rep_$s > /dev/null 2>&1 && echo "IDENTICAL" || echo "MISMATCH"
  done
done
