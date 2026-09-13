# Contributing

Contributions are welcome: bug reports, issues, and pull requests.

## The rule that never bends

The gate is absolute. No change may mark a run proven except a byte-for-byte
comparison against the legacy reference. A pull request that weakens this - a
tolerance flag, a partial comparison, a way to skip validation - is rejected.

## Before you contribute

1. Open an issue first for anything larger than a small fix.
2. Tests run with cargo test; every change to the engine needs the sample
   pipeline (sample/run_all.sh) to stay byte-exact.
3. Every public performance claim must trace to a recorded run in the evidence
   annex. New numbers require a run record.
4. Plain Australian English, no em dashes, numbers over adjectives.

## Licence of contributions

Engine contributions are under AGPL-3.0-or-later; parser contributions under
Apache-2.0. By submitting a pull request you agree to this.
