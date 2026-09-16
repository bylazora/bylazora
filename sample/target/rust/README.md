# batch-txn: rules for whoever writes the logic

The contract is SPEC.json. The gate is absolute: only
`bylazora-core migrate verify` (which calls the validator) marks a run
proven. Run it often; read the diff; a MISMATCH is a diff to fix, never a
threshold to tune. The canon rules live in the agents/ rule pack.
