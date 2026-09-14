# The Evidence Annex

**Licence note:** the engine this paper describes is open source under AGPL-3.0-or-later (its parsers under Apache-2.0); the paper and the evidence are (c) Bylazora.

## Every measurement behind the papers

**Bylazora - bylazora.com** - standing discipline: no public sentence that a run record cannot support.

## 1. The reference workload

A COBOL batch program performing per-account aggregation over transaction files: 55/45 deposit/withdrawal mix, integer cents throughout, two output files (final balances and summary report) with a grand-totals row, LF line endings. Implemented two ways: the shipped Rust engine (bylazora-core, one binary, CPU tier and wgpu GPU tier) and, for measurement provenance, a Python study of six backends (NumPy, NumPy process-parallel, Numba, RAPIDS cuDF, RAPIDS cuPy, plus the compiled GnuCOBOL baseline). Every migrated run, both ways, passes the same byte-exact gate.

## 2. Main benchmark: the shipped Rust engine (final run, 2026-09-13, on the 0.3.2 build; 0.4.1 adds the C ABI, the copybook parser, the DB2 importer, and widens the GPU and CUDA per-account accumulators from 32-bit to 64-bit, a correctness fix with no effect on results at this benchmark's scale - the numbers below are not yet re-measured on 0.4.1 and should be treated as pending confirmation, not as unchanged by assertion)

Speedup versus COBOL (mean of 3 cold-cache reps per cell):

| Scale | COBOL (baseline) | CPU tier (Rust) | GPU tier (wgpu) | GPU tier (CUDA container) |
|-------|-----------------|-----------------|-----------------|---------------------------|
| 1M    | 1.00x           | 26.3x           | 39.5x           | 39.5x                      |
| 10M   | 1.00x           | 48.5x           | 48.5x           | 44.5x                      |
| 100M  | 1.00x           | 25.5x           | 36.1x           | 60.4x                      |
| 1B    | 1.00x           | 28.1x           | 61.3x           | 68.8x                      |

Wall-clock seconds (mean of 3 reps):

| Scale | COBOL | CPU tier (Rust) | GPU tier (wgpu) | GPU tier (CUDA container) |
|-------|-------|-----------------|-----------------|---------------------------|
| 1M    | 0.79  | 0.03            | 0.02            | 0.02                      |
| 10M   | 5.34  | 0.11            | 0.11            | 0.12                      |
| 100M  | 55.59 | 2.18            | 1.54            | 0.92                      |
| 1B    | 601.56| 21.4            | 9.82            | 8.75                      |

Record: every migrated run validated byte-exact against the COBOL reference, all rows IDENTICAL, 0 mismatches (12 wgpu-tier runs and 12 cudarc-tier runs across four scales, means of 3 consecutive reps each, 2026-09-13; idc-4 has no passwordless sudo, so page caches were not explicitly dropped between reps). All three compute tiers share the 8-thread parallel chunked CSV reader with exact i64 accumulation; the wgpu tier is one WGSL atomic kernel with adapter-aware chunking; the CUDA column is the cudarc tier built into the same binary, one atomic CUDA kernel. The RAPIDS container the cudarc tier replaces sits in section 5's provenance table (cuDF column). The MCP server (Rust, stdio) exposes validate and bench with the gate rule intact: no tool can mark a run proven; only the validator can.

## 3. Method

Page cache dropped before every rep (passwordless sudo on the benchmark host); 3 repetitions; wall time from GNU time; every migrated-backend run gated by the byte-exact validator within the harness.

## 4. Hardware

idc-4: Intel i7-14700K (8 performance cores hyperthreaded + 12 efficiency cores = 28 threads), 125 GB RAM, NVIDIA RTX 3080 (8,704 CUDA cores, 10 GB).

Caveat, stated plainly: this is a hybrid consumer CPU. Worker sweeps above 8 threads mix performance cores, efficiency cores, and hyperthread siblings, so high-worker datapoints are conservative. Curves on homogeneous server CPUs are published as they are measured. Cross-machine reproductions run on the idc estate. idc-2 (i9-14900KF, 32 threads, 2x RTX 3090/3090 Ti (10,496/10,752 CUDA cores)) reproduced the study CPU backends byte-exact: 1B numba 101.9s (5.4x), numpy 111.0s (4.9x), parallel 108.4s (5.1x) vs its own COBOL baseline of 549.1s - the ratios match idc-4 within noise while absolute seconds differ by machine. idc-6 (i7-14700KF) runs the study reproduction.

GPU contention, measured twice: on idc-2, a co-tenant 80B model held both GPUs (~46 GB of VRAM), and all 18 RAPIDS runs at 10M+ failed with CUDA OOM on 33 MB allocations. On idc-3, the same backends were device-pinned to a GPU with 4.1 GB free under a co-tenant: all 18 runs completed byte-exact (10M cudf 2.2s, 100M cudf 4.7s, 1B cudf 107.0s; 1B cuPy 96.7s vs 67.6s on a clean 3080 - about 1.4x slower, correct throughout). The mitigation is device pinning plus the chunked reader; dedicated GPU instances remain the sizing answer for production.

## 5. Measurement provenance: the Python study (superseded)

The study that produced the current engine measured six Python backends first. Its tables are retained here as the measurement history, superseded by section 2.

Speedup versus COBOL (mean of 3 cold-cache reps per cell):

| Scale | COBOL | NumPy | NumPy parallel | cuDF | cuPy | Numba |
|-------|-------|-------|----------------|------|------|-------|
| 1M    | 1.00x | 1.1x  | 0.5x           | 0.2x | 0.2x | 0.6x  |
| 10M   | 1.00x | 4.5x  | 2.4x           | 1.4x | 1.4x | 3.1x  |
| 100M  | 1.00x | 8.5x  | 6.6x           | 8.5x | 8.1x | 9.2x  |
| 1B    | 1.00x | 9.3x  | 8.8x           | 7.7x | 8.9x | 10.8x |

Wall-clock seconds (mean of 3 reps):

| Scale | COBOL | NumPy | NumPy parallel | cuDF | cuPy | Numba |
|-------|-------|-------|----------------|------|------|-------|
| 1M    | 0.79  | 0.69  | 1.59           | 3.76 | 3.70 | 1.34  |
| 10M   | 5.34  | 1.20  | 2.25           | 3.95 | 3.90 | 1.71  |
| 100M  | 55.59 | 6.56  | 8.45           | 6.56 | 6.89 | 6.05  |
| 1B    | 601.56| 64.84 | 68.47          | 78.18| 67.63| 55.66 |

Record: 72/72 runs completed ok. 60 migrated-backend runs validated byte-exact against the COBOL reference. 0 mismatches. CPU tier rows above are the polars-read measurement (idc-5, byte-exact); the polars read costs ~0.5s of import overhead, which is why 1M numpy (0.69s) sits above the earlier DuckDB-read number (0.42s) - published as measured.

## 6. Read-engine study (Python era)

A single-threaded pandas parser pinned the study CPU tier near 3.5x at 1B rows. Routing the same backends through DuckDB's parallel CSV scanner lifted them to 4.8-5.1x with no change to the aggregation. The fair bakeoff (idc-5, 1B rows, cold cache, row counts verified): polars read_csv 12.4s, duckdb read_csv_auto 80.2s, pandas read_csv 103.2s. The shipped Rust engine replaced this whole layer with its own 8-thread chunked reader - no third-party read engine, exact integer accumulation - and holds 28.1x at 1B rows (section 2).

## 7. Core-scaling study

Compute-bound variant: per-account 7-day rolling deposit sums over 1B rows, four backends byte-exact against a DuckDB window-function reference, worker sweep 1-28 (idc-6, i7-14700KF, hybrid 8P+12E). Wall seconds, means of 3 reps:

| Path | wall |
|------|------|
| duckdb SQL window (reference) | 489.1s |
| numba (single thread) | 584.8s |
| numpy (vectorized, single thread) | 616.0s |
| parallel w1 | 679.9s |
| parallel w2 | 615.6s |
| parallel w4 | 590.6s |
| parallel w8 | 573.4s |
| parallel w16 | 565.1s |
| parallel w28 | 561.4s |

What the curve says, plainly: worker counts pay until about 8 (679.9s to 573.4s), then flatten - w28 gains only 12s over w8. The remaining time is the sequential parts: the CSV read and the segment machinery. On this hybrid CPU the parallelism ceiling is also physical (performance cores run out at 8 threads). The 196-core question therefore has a measured answer: cores help compute-bound jobs up to a point, then the read engine and the sequential logic decide the floor - which is exactly why the profiling step measures each job rather than extrapolating. A homogeneous server-core sweep is the next datapoint.

## 8. Vendor-neutral GPU (wgpu)

The same per-account aggregation implemented as one WGSL compute kernel on wgpu (Rust host), byte-exact against the COBOL reference at every scale tested. Adapter-aware chunking makes the same binary run anywhere Vulkan/Metal/DX12/WebGPU runs.

| Scale | wgpu groupby kernel | wgpu end-to-end (single-threaded read) | CUDA tier (total job) |
|-------|--------------------:|--------------------------------------:|----------------------:|
| 1M    | 0.001s              | 0.02s                                 | 0.02s                 |
| 100M  | 0.454s              | 1.54s                                 | 0.92s                 |
| 1B    | 7.241s              | 9.82s                                 | 8.75s                 |

Notes: the 1B end-to-end (9.8s, 61.3x) reads the CSV on 8 parallel threads in bounded 8M-row chunks with per-chunk dispatch (shipped in 0.3.1; the single-threaded read it replaced was 50.5s). The CUDA column is the cudarc tier (0.3.2): the same parallel read plus one atomic CUDA kernel, 8.75s and 68.8x at 1B. Peak memory is about 8 threads x 96 MB of row buffers, so the same binary still runs on contended boxes with a few GB free. The same binary also ran byte-exact on llvmpipe (software Vulkan, no GPU). The Intel iGPU was not enumerable headless; the next non-NVIDIA hardware datapoint is an AMD card. Performance deltas versus CUDA are inputs to the profiling step, exactly like every other tier.

## 9. Correctness trap corpora

Three planted corpora prove the gates catch the failure classes. (1) Float trap: group sums engineered to cross 2^53 - the float64 aggregation rounds, the rubric refuses the path up front, and the byte-exact validator flags any output that shipped the rounded values. (2) Look-ahead trap: inputs containing future rows, correct answer defined by trailing-only windows - a centered-window implementation fails the gate. (3) Reproducibility check: any repetition that differs from rep 1 marks the run nondeterministic, warm and cold cache runs must agree, and seeds are fixed. All three ship as tests in the library (12/12 passing, 2026-09-12).

## 10. Ratios, not prescriptions

The numbers above are ratios against a commodity-rehosted COBOL baseline; absolute seconds are machine-specific. Two workload shapes (per-account aggregation and 7-day rolling windows), four scales to one billion rows, and a worker sweep make up the measured ratio space across CPU, cores, and GPU. The recommendation that follows from them is always the same: profile the estate, then choose - infrastructure and cloud options, job complexity, batch size, and team balance decide the target state. The harness supplies the measurement, never the directive.

## 12. Failure-case mapping

The named failures and their mechanisms are mapped in Why mainframe migrations fail (site/why-migrations-fail.html): TSB 2018 (inspection, not proof), KCB 2024 (data fidelity), Suncorp (big-bang write-off), Westpac (unmeasurable risk), plus the industry 70% forecast and the stay-side cost evidence. Each mechanism cited there is a gate shipped in this library.

## 13. Claims audit

Every figure on the website and in the papers traces to this annex. Dated additions here supersede the papers.
