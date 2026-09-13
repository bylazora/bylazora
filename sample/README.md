# Bylazora: a sample mainframe migration, with proof

A complete, runnable example of a COBOL batch application migrated to the
Bylazora engine. This repository is the public proof artifact: clone it,
run one script, and watch the byte-exact gate certify the migration.

## What is inside

- **legacy/** - the COBOL batch application (BATCHTXN.cob, its copybook, its
  build script) plus a small fixed-seed dataset (100,000 transactions).
- **engine/** - the migrated application: the Bylazora engine as a
  prebuilt Linux x86_64 binary (CPU tier and vendor-neutral wgpu GPU tier).
  Source available under licence - see engine/NOTICE.md.
- **results/** - the COBOL reference outputs and the migrated outputs, after
  a run.
- **run_all.sh** - the whole pipeline: compile and run the legacy reference,
  run the migrated tiers, then hold every output to the byte-exact gate.
- **.github/workflows/ci.yml** - the gate as a regression test: every push
  re-runs the migration and fails on a single differing byte.

## Run it

```
sudo apt-get install -y gnucobol   # the legacy compiler
bash run_all.sh
```

Expected output: each tier prints timing, then the gate prints IDENTICAL for
the CPU tier, the GPU tier (where a GPU or software Vulkan is available),
and the CUDA tier (where an NVIDIA GPU and driver are available).

## Why this is the whole argument

Mainframe migrations fail because they are validated by inspection. This
repository demonstrates the alternative: the migrated application must
reproduce the legacy output byte for byte, and one differing byte fails the
gate. The same gate runs in CI here and in production engagements - this
sample is a regression test for the capability, not a marketing mock.

Scaling: the dataset is small so the example runs anywhere. The same engine
is measured byte-exact at one billion rows (see the evidence annex at
bylazora.com/evidence.html).

Licence: see LICENSE.md. The Developer tier is free: clone it, run the
pipeline, and let your agents draft against the gate, for jobs up to 10
million rows each. Buy a Pro key when a workload goes to production or
exceeds the free cap; Enterprise terms cover engagements and integrators.
