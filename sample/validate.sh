#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
BIN=engine/bylazora-core
if [ ! -x $BIN ]; then echo "engine binary missing or wrong platform; source available under licence"; exit 1; fi
$BIN validate --ref-dir results/legacy --other-dir results/cpu
