# The Engine

**Licence note:** the engine this paper describes is open source under AGPL-3.0-or-later (its parsers under Apache-2.0); the paper and the evidence are (c) Bylazora.

## A mainframe migration library with byte-exact proof

**Bylazora - bylazora.com**

---

## Executive summary

Mainframes still run the world's most important batch processing, and they do it reliably. What is failing around them is the operating model: the COBOL workforce is retiring at roughly 10% a year, mainframe software pricing keeps rising, and vendor end-of-support dates are turning a cost problem into a deadline.

Every traditional exit is validated by inspection. Rewrites are re-certified by eye. Conversions are trusted to an automation percentage. Rehosts keep the problem and move the licence. Inspection is why migrations fail: a UK bank's 2018 migration locked 1.9 million customers out and cost over GBP 200 million to fix.

Bylazora built the exit as a product. **The Engine** is a library that makes migration acceptance mechanical: define a job once, run it on the legacy reference and on every modern backend, and the validator requires the outputs to be identical, byte for byte. One differing byte fails the job. The benchmark harness measures each tier and reports the measured ratios between CPU and GPU across data volumes and workload complexity. Those ratios are inputs to profiling, not verdicts: the target state is chosen by profiling your estate, balancing the infrastructure and cloud services available, the complexity of each job, and the size of each batch. We do not direct where a workload goes.

And the Engine is not the test at the end of the migration - it is the runtime of the new solution. The migrated jobs execute on the same engine that validated them, and the gate is the safety system of the new estate. Proof is not a phase; it is the execution model.

The ratios, measured on commodity hardware: at one billion rows the cudarc CUDA tier aggregated 68.8x faster than the compiled COBOL baseline end to end (8.75s against 601.6s), the vendor-neutral wgpu tier 61.3x (9.8s), and the CPU tier (Rust) 28.1x (21.4s) - and at one million rows the CUDA tier loses while wgpu stays ahead. The same wgpu binary is byte-exact on NVIDIA, and on software Vulkan with no GPU at all - it runs on AMD, Intel, Apple, and the web by construction. The absolute seconds are machine-specific; the ratios are what travel, and profiling on your estate calibrates both. Every number traces to a recorded harness run.

Free while you migrate: the Developer tier is the full engine with the gate, capped at 10 million rows per job. A Pro key (USD 4,900/year per production team) unlocks unlimited scale and production use, including client engagements for shops of any size. Enterprise terms cover estate programs, the dashboard, support, and redistribution. Or have us run it for you.

The origin story: before the migration engine, Bylazora built quant and ML infrastructure at scale and made every one of these mistakes the expensive way. The library is the aftermath - each lesson became a machine-checkable gate rather than a coding standard. It does not ask your team to be careful; it refuses to certify anything else.

## 1. What is in the box

- The JobSpec contract. One file per migrated job: inputs and schemas, the byte-level output contract (exact headers, field widths, ordering, totals rows, line endings), the run interface, and the validation and benchmark configuration. The same spec drives discovery, rebuild, test, and shadow run.
- The byte-exact validator. Compares every declared output file against the legacy reference, byte for byte, with a diff attached to any mismatch. A missing reference means unvalidated, never proven.
- The benchmark harness. Cold-cache repetitions, backend dispatch, per-run validation gating, and machine metadata recorded with every result.
- The report generator. Speedup tables, log-scale charts, per-run records: the evidence your steering committee sees.
- Backend templates. GnuCOBOL rehost and the Rust engine: CPU tier, wgpu GPU tier, cudarc CUDA tier, each an independent implementation of the same contract, instantiable per job.
- Estate intake. The copybook parser (P1) turns COBOL record layouts into schemas with fixed-point fidelity, and the DB2 importer (P2) reads catalog schemas and DEL unloads into canonical inputs: money arrives as integer cents, never float.
- Seeded synthetic generators. The DB2-unload pattern for building deterministic test corpora.
- CLI, native binaries, container. spec init, validate, benchmark, and report on the command line; a single native binary for the CPU tier; a pinned container for the GPU tier.
- The self-hosted dashboard (rolling out). Dual-run shadowing, continuous reconciliation, per-run equivalence evidence: in your environment, not ours.

## 2. The gate

Automation percentages are claims about effort. Equivalence is a claim about correctness, and it is mechanical:

- Exact arithmetic. COBOL business math is fixed-point. The engine carries money as integer cents end to end, provably exact, not approximately close.
- Byte-exact outputs. Field values, totals, ordering, formatting, line endings, all identical, or the job fails with a diff.
- Continuous enforcement. The gate runs in CI, in benchmarks, and during shadow runs. Divergence cannot silently accumulate.

This is the property no major migration vendor publishes. It is the difference between 'we converted your code' and 'we can prove your system still works'.

The gate exists because of three failure classes we lived through in our own quant and ML infrastructure: floating-point drift (float intermediates corrupt exact money), future-data leakage (windows that see tomorrow invalidate every downstream calculation), and non-idempotent runs (the same job returning different numbers). Each is neutralised by construction: the validator's byte comparison has no tolerance; backends run as pure functions in fresh processes with trailing-only windows; the harness byte-compares every repetition, warm and cold. Planted trap corpora - a float trap that crosses 2^53, a look-ahead trap with future rows, a reproducibility check - ship with the library and prove the gates catch what they claim.

## 3. The evidence

The reference workload: a COBOL batch program performing per-account aggregation over transaction files, running on the shipped Rust engine across three tiers and benchmarked at four scales up to one billion rows, three cold-cache repetitions each, every output byte-identical to the COBOL reference.

| Scale | COBOL (baseline) | CPU tier (Rust) | GPU tier (wgpu, any GPU) | GPU tier (CUDA) |
|-------|-----------------|-----------------|--------------------------|------------------|
| 1M    | 1.00x           | 26.3x           | 39.5x                    | 39.5x            |
| 10M   | 1.00x           | 48.5x           | 48.5x                    | 44.5x            |
| 100M  | 1.00x           | 25.5x           | 36.1x                    | 60.4x            |
| 1B    | 1.00x           | 28.1x           | 61.3x                    | 68.8x            |

All rows byte-exact against the COBOL reference, measured on the shipped Rust engine (0.3.2). The cudarc CUDA tier leads: 8.75s at 1B, after the parallel chunked host read shipped in 0.3.1 removed the single-threaded read bound. The wgpu tier holds 61.3x on any GPU or none; the CPU tier needs no GPU and holds 28.1x. The CUDA column is the cudarc tier built into the same binary; the RAPIDS container it replaces is retained as the escape hatch for arbitrary jobs.

Three honest conclusions. GPU acceleration is transformative at scale - the cudarc CUDA tier holds 68.8x at one billion rows on a single consumer RTX 3080 (8,704 CUDA cores) and the vendor-neutral wgpu tier 61.3x on the same card, consistent with independent industry data (NVIDIA reports cuDF accelerating pandas workloads by up to 150x on GB-scale ETL; AWS reports up to 3.7x for GPU-accelerated Spark; TPC-H GPU query engines report 7.5x and higher). At small scale the RAPIDS container the cudarc tier replaces lost - 0.2x at one million rows - because transfer and launch overheads exceeded compute savings; those numbers are provenance now. And the read path matters as much as the compute: the same 8-thread parallel chunked reader that holds the CPU tier's 28.1x at a billion rows now drives both GPU tiers, lifting the 1B row from 50.5s to 9.8s (wgpu) and 8.75s (cudarc).

We publish the crossover because it feeds profiling, not prescriptions: the ratios say what is worth testing on each tier. The migration team then balances them against the infrastructure and cloud services available, the complexity of the jobs, and the batch sizes, and chooses the best value-for-money target state. No client pays for acceleration that does not accelerate.

A core-scaling study extends the evidence: the harness measures worker-count sweeps and a compute-bound rolling-aggregation workload, with machine metadata recorded so every curve carries its caveats. A GPU-contention study measures both failure (a co-tenant model holding all VRAM) and mitigation (device pinning on a 4 GB free GPU - byte-exact throughout). The benchmark boxes are hybrid consumer CPUs (8 performance cores plus 12 efficiency cores); curves from homogeneous server CPUs are published as they are measured. See the Evidence Annex for the full record.

## 4. Three tiers, profiled to fit

- Keep-COBOL tier: GnuCOBOL on commodity instances, for logic-dense, low-volume jobs where the language is not the bottleneck.
- CPU tier: the Rust engine with a parallel chunked reader and exact integer accumulation; needs no GPU and holds 28.1x at 1B.
- GPU tier: two paths in the same binary. The cudarc CUDA kernel on NVIDIA (68.8x at 1B) or the wgpu kernel set for any GPU (61.3x), covering NVIDIA, AMD, Intel, Apple, and the web. The wgpu path removes the GPU vendor from the lock-in question entirely; the RAPIDS container is retained for arbitrary jobs.

The harness measures each job on the candidate tiers and hands the migration team the numbers. The team decides, balancing infrastructure and cloud options, job complexity, batch size, and cost. The library provides the measurement; the team owns the decision.

## 5. How you run it

- Bylazora Developer (free). The full engine, the gate, and the MCP interface for evaluation and migration trials, capped at 10 million rows per job. No key, no form, no telemetry.
- Bylazora Pro (USD 4,900/year per production team). One named organisation, unlimited scale, production use of its own and its clients' workloads, including installs on client estates. Every run names the licensee, and the key's signed expiry ends engagement access unless renewed. An Ed25519-signed key, verified locally (bylazora-core licence set, or BYLAZORA_KEY); no network, no telemetry.
- Bylazora Enterprise (order form). Estate-scale programs, support SLAs, and embedding or redistributing the engine. Shops of any size, including one-person boutiques, run client engagements on a Pro key. Acceptance criteria in your contracts stop being opinions and become byte comparisons. The governed seven-phase service (The Governed Exit) runs on the same engine.

On AI: in our engagements, AI drafts specifications, translations, and tests; humans sign; the gate disposes. The library ships no model and no telemetry. Its AI interface is an MCP server plus a JSON contract over the same CLI - your assistant can operate the engine, propose, and measure. It cannot mark a job proven. Only the validator can.

That boundary is the answer to vibe-coded migrations. The gate verifies outputs, not provenance: it does not care whether the migrated code was hand-written or agent-drafted, because a hallucination produces different bytes, and different bytes fail the gate with a diff before anything can ship. A migration has something greenfield coding does not - an oracle. The legacy system is the ground truth, and the migrated system is certified against it, never against the agent's confidence. The agent is free to hallucinate while drafting; the gate disposes.

## 6. Where the engine sits in the migration

The engine is not the test at the end of a migration. It is the runtime of the new solution, and the gate is part of how it runs.

1. **Specify.** Each job in the estate becomes a job specification - inputs, byte-level output contract, run interface. The specification is the migration's working document.
2. **Capture the golden reference.** The legacy job's outputs, totals, and timings are captured once as the byte-level reference. That reference is the contract everything is built against.
3. **Build inside the gate.** The migrated implementations are developed as backends of the engine. Every iteration runs through the validator - a developer gets a byte diff, not a hunch. The engine is the build tool, not just the test tool.
4. **Route by benchmark.** The harness measures each backend on the candidate hardware, and the profiling step chooses the target state from those measurements.
5. **Cut over, running on the engine.** In production, the migrated jobs execute as engine backends - the same binary that validated them runs them. During dual-run, the gate reconciles every output against the legacy system, continuously.
6. **Stay proven.** Every nightly run passes through the gate. Drift, data change, or regression fails loudly with a diff, and the evidence accumulates into the standing archive.

The migrated system does not merely pass a test once. It runs on the engine that tests it - and the gate is the safety system of the new estate.

## 7. What stays ours

The code ships. The know-how stays: the migration playbook, the benchmark corpora, the per-client replication recipes, and the equivalence-proof rubric - the numeric-exactness analysis that says which datatypes are provably safe. The licence terms forbid using the Engine to operate a competing migration service. That boundary is how a licensee gets full access to the tool without getting the business - and it is stated openly so a buyer's legal review has nothing to discover.

## 8. The roadmap

Every addition lands behind the same byte-exact gate.

| **Ships today** | **Estate intake (next)** | **Self-hosted reconciliation** | **AI-assisted translation** |
|---|---|---|---|
| The engine core: JobSpec contract, validator, harness, backend templates, CLI, and the MCP interface | Read copybooks, DB2 schemas, and JCL directly into job specifications - record layouts, fixed-point fields, batch windows and dependencies - a real estate specified in hours, not weeks | Dual-run shadowing with a self-hosted reconciliation dashboard, drilling down to the byte diff behind any mismatch | In our engagements: AI drafts translations and test suites; humans sign, the gate disposes, and the gate-as-judge harness scores every draft byte-for-byte before it is considered |

## 9. Getting started

The package ships with the worked example from this paper. On an afternoon, on your hardware, you can reproduce the published numbers and watch the gate reject a deliberately broken backend. For a client estate, that afternoon is the proof trial: one representative job, reproduced byte-exact and benchmarked on your hardware, with the trial fee credited in full against the first migration wave.

Contact: bylazora.com - hello@bylazora.com

---

*All figures are measurements from recorded harness runs on the published benchmark machines; machine metadata is recorded with every run. Bylazora does not promise a percentage. It promises a measurement.*
