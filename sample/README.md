# A sample mainframe migration, with a sealed reference and a byte-exact gate

A complete, runnable example of one COBOL batch job migrated into a
`bylazora-core migrate` workspace. The legacy program's output is sealed as
the reference, the migrated application in Rust must reproduce it byte for
byte, and the gate refuses anything else. The dataset is synthetic; this
sample demonstrates the routine, not a production estate.

## The shape

- `SPEC.json`: the job contract. Name, declared inputs and outputs, the
  copybook schema, and the sealed reference hash.
- `legacy/`: the legacy side: BATCHTXN.cob, its copybook, its build
  script, and the fixed-seed dataset (100,000 transactions, 10,000
  accounts).
- `input/`: the declared inputs, ready for the migrated application.
- `reference/`: capture.sh, which compiles and runs the legacy COBOL with
  GnuCOBOL, and the sealed output/ with its manifest. This is the contract.
- `target/rust/`: the migrated application, written into the skeleton
  that `migrate new` generated.
- `runs/`: the append-only verdict trail from `migrate verify`.
- `verify.sh`: one command that runs the whole gate.

## Run the gate

From the repository root:

```
cargo build --release --manifest-path engine/Cargo.toml
engine/target/release/bylazora-core migrate verify sample
```

Expected output: the target builds, runs on input/, the validator compares
its output against the sealed reference, and the gate prints IDENTICAL.
The verdict is appended to sample/runs/.

To see the contract and the latest verdict:

```
engine/target/release/bylazora-core migrate status sample
```

## Re-seal the reference

The sealed bytes are committed, so the gate runs without GnuCOBOL. To
re-derive them from the COBOL source (the repository's CI does this on
every push and asserts the re-derived hash matches the committed seal):

```
sudo apt-get install -y gnucobol
engine/target/release/bylazora-core migrate reference sample --force
```

## The loop for a coding agent

1. Read SPEC.json, then reference/output/ to see the exact bytes to
   reproduce.
2. Change target/rust/src/main.rs.
3. Run `engine/target/release/bylazora-core migrate verify sample`.
4. Read the diff; a MISMATCH is a diff to fix, never a threshold to tune.
5. Repeat until the gate reports IDENTICAL.

## Limits, stated plainly

The data is synthetic and small (100,000 rows) so the example runs
anywhere. This sample proves the routine: seal, migrate, gate. It does not
prove the engine's throughput; the billion-row measurements live in the
evidence annex at bylazora.com/evidence.html.

Licence: this sample lives in the bylazora repository, AGPL-3.0-or-later
for the engine and its workspace, Apache-2.0 for the parsers. See the
repository root for the full texts.
