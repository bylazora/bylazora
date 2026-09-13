# The Governed Exit

**Licence note:** the engine this paper describes is open source under AGPL-3.0-or-later (its parsers under Apache-2.0); the paper and the evidence are (c) Bylazora.

## Migrating COBOL Batch Workloads from the Mainframe to a GPU-Accelerated Cloud, With Proof at Every Step

**Bylazora - Mainframe Migration Services**
bylazora.com

---

## Executive summary

Mainframes are not dying of natural causes. They process an estimated 68% of the world's production IT workloads at roughly 6% of its IT cost, and IBM reports that 70% of its mainframe clients are still growing their MIPS capacity. What IS dying is the operating model around them: the COBOL workforce is retiring at roughly 10% per year while an estimated 220-800 billion lines of COBOL remain in production, and mainframe software pricing keeps rising (z16 MIPS pricing up an estimated 15-20% since 2022).

Organizations are not short of migration vendors. They are short of vendors who can PROVE the migration is safe. Every major player markets an automation percentage - but an automation rate is a claim about effort, not about correctness. The industry's cautionary tales (a UK bank's 2018 migration locked 1.9 million customers out and cost over GBP 200 million to remediate) exist because migrated systems were validated by inspection, not by proof.

Bylazora takes a different position: **every workload we migrate ships with byte-exact equivalence evidence against its legacy reference, and with benchmark evidence for the target it runs on.** We do not ask a client to trust a conversion percentage. We hand them the proof. The method is implemented as a licensable library - The Engine - which your own team or SI can run under the same gate; this paper describes the cycle that library executes and the service that runs it for you.

## 1. The problem

Three forces make the mainframe an increasingly difficult place to stay:

1. **The skills cliff.** The average COBOL developer is in their late 50s, and roughly 10% of the workforce retires each year, with no replacement pipeline (around 70% of universities no longer teach COBOL). The talent risk compounds yearly.
2. **The cost trajectory.** Mainframe software is the largest line item - typically 30-50% of the total mainframe budget - and large estates pay roughly USD 1,000-2,000 per MIPS per year, with pricing trending up, not down.
3. **The exit risk itself.** Migration is the highest-stakes project an enterprise runs. Failed migrations are existential (the 2018 UK bank case); successful ones still commonly run 3-7 years with dual environments in parallel.

To be clear about what is NOT the problem: mainframe reliability and raw cost-per-workload. The z16 platform advertises extraordinary hardware uptime, and mainframes process enormous volumes for their spend. A credible migration argument rests on the skills cliff, the licensing trajectory, and the strategic cost of lock-in - not on claims that the mainframe is broken.

## 2. Why traditional migration approaches fail

**Rip-and-replace rewrites** discard decades of battle-tested business logic and replace it with new code that must be re-certified. A US state government estimated a rewrite at USD 200 million with 5-10 years of federal re-certification; an automated refactor completed instead in 18 months. Rewrites also maximize the chance of behavioral drift: the new system is 'equivalent' only by interpretation.

**Automated conversion without equivalence proof** converts the language but cannot demonstrate the behavior survived. An automation rate of 99.7% still leaves thousands of lines to reconcile by hand - and says nothing about whether the other 99.7% is correct.

**Rehosting in place** (emulation) moves the problem rather than solving it: the COBOL remains COBOL, the skills cliff remains, and per-core emulator licensing replaces per-MIPS mainframe licensing - cheaper, but not modernized.

Gartner's forecast that 75% of 'mainframe exit' vendors will pivot or cease by 2030 reflects the market's frustration with these approaches. The opportunity belongs to whoever can make migration verifiable.

## 3. Our approach: the governed migration cycle

The same seven-phase cycle runs inside the Engine library and in our service engagements. The phases:

1. **Grab** - We take custody of the client's code and documentation: COBOL source, copybooks, JCL/PROC libraries, data dictionaries, and run statistics. Nothing is re-keyed; everything is inventoried with provenance.
2. **Review** - Static analysis and cross-reference build the function inventory: what each program does, what it touches, what depends on it, and what is actually dead code. This phase produces the migration specification.
3. **Rebuild** - Each function is re-implemented on its classified target tier (see Section 5) with business logic preserved one-to-one in semantics. The data-access layer changes; the behavior cannot.
4. **Deploy** - Rebuilt functions land on the target platform as containerized jobs orchestrated by a scheduler that preserves batch windows and dependencies (the new JES).
5. **Migrate data** - DB2 tables, VSAM files, and QSAM/GDG datasets move to the canonical target formats with fixed-point decimal semantics preserved (no float drift - money stays exact).
6. **Test** - This is where we differ from the market. Every rebuilt function must reproduce its legacy reference output BYTE-FOR-BYTE. The equivalence gate runs continuously - not once at acceptance - and any divergence fails the job automatically.
7. **Roll over** - Dual-run shadow operations against production, reconciliation via the equivalence gate, then cutover. Decommissioning proceeds job by job, so risk is bounded to one function at a time.

## 4. The equivalence guarantee

Automation percentages are claims about effort. Equivalence is a claim about correctness, and it is the only claim that protects a client's business logic.

Our methodology enforces three properties:

- **Exact arithmetic.** COBOL business math is fixed-point (PACKED-DECIMAL). We carry money as integer cents end-to-end and carry money as integer cents end-to-end, so the modern implementation is provably exact - not approximately close.
- **Byte-exact outputs.** The migrated function must produce output identical byte-for-byte to the legacy reference across every test dataset, including totals and ordering. A one-byte difference is a failure.
- **Continuous enforcement.** The equivalence check is wired into our benchmark harness, so every change, every scale-up, and every re-run is automatically re-validated. Divergence cannot silently accumulate.

This is the property no major migration vendor publishes. It is the difference between 'we converted your code' and 'we can prove your system still works'.

## 5. The target technology: three tiers, chosen by evidence

Not every workload should leave COBOL, and not every workload should touch a GPU. We benchmark before we choose, and we publish the crossover data instead of hiding it.

**Our measured reference workload** - a representative COBOL batch program performing per-account aggregation over transaction files, running on the shipped Rust engine across three tiers and benchmarked at 1M, 10M, 100M, and 1B rows (3 cold-cache repetitions each, all outputs byte-identical to the COBOL reference):

| Scale | COBOL (baseline) | CPU tier (Rust) | GPU tier (wgpu, any GPU) | GPU tier (CUDA) |
|-------|-----------------|-----------------|--------------------------|------------------|
| 1M    | 1.00x           | 26.3x           | 39.5x                    | 39.5x            |
| 10M   | 1.00x           | 48.5x           | 48.5x                    | 44.5x            |
| 100M  | 1.00x           | 25.5x           | 36.1x                    | 60.4x            |
| 1B    | 1.00x           | 28.1x           | 61.3x                    | 68.8x            |

All rows byte-exact against the COBOL reference, measured on the shipped Rust engine.

Three honest conclusions follow. First, GPU acceleration is transformative at scale - the cudarc CUDA tier holds 68.8x at 1B rows on a single consumer RTX 3080 (8,704 CUDA cores) and the vendor-neutral wgpu tier 61.3x on the same card, consistent with independent industry benchmarks for GPU data processing (NVIDIA reports cuDF accelerating pandas workloads by up to 150x on GB-scale ETL; AWS reports up to 3.7x for GPU-accelerated Spark). Second, at small scale the RAPIDS container the cudarc tier replaces LOST - 0.2x at 1M rows, because data-transfer and kernel-launch overheads exceeded the compute savings; those numbers are provenance now. Third, the read path matters as much as the compute: the same 8-thread parallel chunked reader that holds the CPU tier's 28.1x at 1B rows now drives both GPU tiers, lifting the 1B row from 50.5s to 9.8s (wgpu) and 8.75s (cudarc). The crossover remains the router: benchmark, then choose.

That crossover feeds profiling, not prescriptions: the ratios say what is worth testing on each tier. The migration team balances them against the infrastructure and cloud services available, job complexity, and batch sizes, and chooses the best value-for-money target state. The three tiers it chooses between:

- **Keep-COBOL tier** (GnuCOBOL on cloud instances) for logic-dense, low-volume jobs where the language is not the bottleneck.
- **CPU tier** (the Rust engine with a parallel chunked reader and exact integer accumulation) - 28.1x at 1B with no GPU at all.
- **GPU tier** (the wgpu kernel set on any GPU, or the cudarc CUDA kernel on NVIDIA, both in the same binary) for large tabular and aggregation workloads - 68.8x at 1B on cudarc, 61.3x on wgpu; the RAPIDS container is retained for arbitrary jobs.

## 6. The target architecture

The to-be platform is a governed, cloud-native batch estate:

- **Data:** canonical Parquet datasets on object storage, preserving fixed-point semantics; keyed storage for VSAM-class access patterns; versioned objects for generation data groups.
- **Compute:** containerized jobs on three instance tiers (general-purpose for keep-COBOL, CPU-optimized for the Rust CPU tier, GPU instances for accelerated jobs), scheduled by a DAG engine that preserves batch windows and dependencies.
- **Scale-out for large estates:** workloads shard by business key (account, customer, policy) - never by time - so inter-row semantics stay intact per shard; multi-GPU and multi-node execution via sharded wgpu kernels or the RAPIDS container path; reduction steps are small and cheap, so scaling is near-linear. Commodity hardware is the point: the exit from z/OS should not require an entrance into NVIDIA. On our benchmark box, one NVIDIA RTX 3080 handled 100M rows of this workload class in 6.6 seconds and 1B rows in 68 seconds; estates that exceed a single GPU's memory are partitioned, not capped.
- **Economics:** batch is periodic and checkpointable, so spot capacity and commitment plans cut steady-state compute materially; a representative nightly 100M-transaction batch on a dedicated-GPU target prices in the low hundreds of dollars per month.

## 7. Indicative economics

**Target platform cost (worked example):** a nightly batch of 100M transactions, 3 hours of daily processing, one L40S-class GPU instance (18,176 CUDA cores) plus one CPU instance, 2 TB of object storage, and 200 GB of monthly egress prices at roughly USD 380/month on AWS on-demand - before spot or commitment discounts. Against even a conservative USD 3,000-8,000/month mainframe allocation, that is an 87-95% reduction, and the published industry outcome range is consistent (30-50% three-year TCO reduction with a ~22-month median payback across modernization programs generally; headline cases report up to 70-90% with named clients).

**Engagement model:** fixed-fee and value-anchored - discovery, per-job migration tiers, and optional managed-run support are quoted against the assessed function inventory, not hours, and shared at proposal stage. Every engagement's acceptance criterion is the same: byte-exact equivalence and the benchmark evidence.

## 8. Getting started

A discovery engagement typically runs 2-6 weeks per workload cluster and produces: the function inventory with hot-path ranking, the data-interface catalog (entry and exit points), a mainframe-replication test environment description, and a fixed-fee migration proposal with per-job equivalence commitments. There is no code change to the client's production systems during discovery. Teams that prefer to run the cycle themselves license The Engine; the gate and the evidence are the same either way.

Contact: bylazora.com

---

*Figures cited from public sources (MarketsandMarkets, IBM, Gartner, Barclays, ITIC, Deloitte, and others, 2023-2026) and from Bylazora's own benchmark measurements. All savings figures are ranges or named cases; Bylazora does not promise a percentage - it promises a measurement.*
