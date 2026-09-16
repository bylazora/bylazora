#!/usr/bin/env bash
# Runs the full gate for this job: build the target, run it on the declared
# inputs, and byte-compare its outputs against the sealed reference.
set -euo pipefail
exec bylazora-core migrate verify "$(cd "$(dirname "$0")" && pwd)"
