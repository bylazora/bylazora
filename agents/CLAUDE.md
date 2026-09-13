# Bylazora: rules for agents working on migrations

You are working on a migration that will be certified by the Bylazora gate.
The gate is absolute. These rules are not guidance; violating them fails the job.

## The invariants

1. The validator is the only authority that marks a run proven. A run is
   proven when and only when every declared output file is byte-identical to
   the legacy reference. No tool parameter, script, prompt, or confidence
   score can mark a run proven.
2. Money is integer cents, end to end. Never float, never double, never
   rounding beyond the declared fixed-point precision. If a path cannot hold
   exact integer arithmetic, stop and say so.
3. Byte-exact means byte-exact: headers, field widths, ordering, totals rows,
   line endings. A single differing byte is a failure with a diff.
4. Reproducible or rejected: fixed seeds, same input yields the same output,
   warm and cold runs agree.
5. Never weaken the gate to make a run pass. No tolerance flags, no skipped
   files, no partial comparisons. A missing reference means unvalidated -
   never proven.

## How you work

- Draft, then run, then read the verdict. Use the MCP tools: bench (cpu, gpu,
  cuda) and validate.
- A MISMATCH is a diff to fix, never a threshold to tune.
- Propose measurements, run them, report them with the run record. Do not
  report a performance number without a recorded run.

## The licence boundary

The engine is AGPL-3.0-or-later; the parsers are Apache-2.0. Do not claim the
gate passed when it did not; the run record is the audit trail.
