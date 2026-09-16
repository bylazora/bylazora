#!/usr/bin/env bash
# The reference capture: compile the legacy COBOL with GnuCOBOL and run it
# on the declared inputs. `bylazora-core migrate reference` executes this
# script and seals whatever it leaves in output/ as the byte-level contract.
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p output
rm -f output/*
cobc -free -x -O2 -o /tmp/batch-txn-reference ../legacy/BATCHTXN.cob
/tmp/batch-txn-reference ../input/transactions.csv ../input/balances.csv output
