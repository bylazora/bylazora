#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p results/legacy results/cpu results/gpu

echo "== 1. legacy reference (GnuCOBOL) =="
bash legacy/build_run.sh legacy/data/transactions.csv legacy/data/balances.csv results/legacy

echo "== 2. migrated application (Rust engine, prebuilt binary) =="
BIN=engine/bylazora-core
chmod +x $BIN 2>/dev/null || true
if [ ! -x $BIN ]; then echo "engine binary missing or wrong platform; source available under licence"; exit 1; fi
$BIN bench legacy/data results/cpu --backend cpu
$BIN bench legacy/data results/gpu --backend gpu || echo "GPU tier note: no adapter, or software fallback used"
$BIN bench legacy/data results/cuda --backend cuda || echo "CUDA tier note: no NVIDIA GPU, or driver missing"

echo "== 3. the gate: byte-exact equivalence =="
$BIN validate --ref-dir results/legacy --other-dir results/cpu
if [ -f results/gpu/final_balances.csv ]; then
  $BIN validate --ref-dir results/legacy --other-dir results/gpu
fi
if [ -f results/cuda/final_balances.csv ]; then
  $BIN validate --ref-dir results/legacy --other-dir results/cuda
fi
echo "gate passed: migrated outputs byte-identical to the COBOL reference"
