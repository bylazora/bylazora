# Modernizing the Mainframe

**Licence note:** the engine this paper describes is open source under AGPL-3.0-or-later (its parsers under Apache-2.0); the paper and the evidence are (c) Bylazora.

## A Comprehensive White Paper on Bylazora's Governed, AI-Enabled Migration Service

**Bylazora | bylazora.com**

---

## Executive summary

Every enterprise running COBOL faces the same three questions: Should we leave COBOL, or take it with us? If we move, how do we survive the move? And once we are on the new platform, how do we handle the days when demand spikes to ten times normal?

Bylazora exists to answer all three with evidence instead of opinion. We are a turnkey mainframe-migration service that takes custody of a client's code and documentation, reviews it, rebuilds it, deploys it, migrates the data, tests it, and rolls it over - under a governance model whose acceptance criterion is byte-exact equivalence with the legacy system, not a conversion percentage. We offer every destination path: move off COBOL entirely, keep COBOL and rehost it to the cloud, or take the AI-enabled route in between. The workload's own characteristics decide which path it takes. The equivalence gate makes every path safe. And the target architecture is built to scale elastically, so peak days - tax time for a revenue agency, month-end for a bank - are a capacity-planning exercise, not a prayer.

## 1. The mainframe today: three uncomfortable facts

**Fact one: the mainframe is not going away by itself.** Mainframes process an estimated 68% of the world's production IT workloads at roughly 6% of its IT cost, run 90% of credit-card transactions, and serve 71% of the Fortune 500. IBM reports that 70% of its mainframe clients are still growing MIPS. The platform works.

**Fact two: the operating model around it is failing.** An estimated 220-800 billion lines of COBOL remain in production. The average COBOL developer is in their late 50s, roughly 10% of the workforce retires annually, and about 70% of universities no longer teach the language. Mainframe software - typically 30-50% of the total mainframe budget - keeps getting more expensive (z16 MIPS pricing up an estimated 15-20% since 2022). The skills cliff is compounding, and it compounds against a growing workload.

**Fact three: migration is where the industry's trust was lost.** A UK bank's 2018 migration locked 1.9 million customers out of their accounts and cost more than GBP 200 million to remediate. Gartner forecasts that 75% of 'mainframe exit' vendors will pivot or cease to exist by 2030. Clients have been promised conversions, percentages, and savings - and handed regressions, rework, and risk.

The market is large and growing regardless: application-modernization services are roughly a USD 22.7 billion market (2025) growing at 15-17% annually toward USD 51-70 billion by 2031-32. The client demand is real. What is missing is trust - and trust is a measurement problem, not a marketing problem.

## 2. The three paths - and how we choose

There are three defensible ways off a mainframe, and Bylazora delivers all three. The choice is made per workload by evidence, never by ideology.

### Path A: Move off COBOL (modernize)

The business logic is re-implemented in a modern stack - a Rust core with parallel chunked reads and GPU-native kernels - with the data-access layer rebuilt against cloud-native stores. This removes the COBOL skills dependency entirely and unlocks modern tooling, AI integration, and the performance tiers described in Section 5. It is the right path for high-volume, well-understood batch and transaction workloads whose semantics can be pinned down by tests. Its historical risk - behavioral drift - is what our byte-exact equivalence gate eliminates: the new implementation must reproduce the legacy output byte-for-byte, on every dataset, forever.

### Path B: Migrate COBOL to the cloud (rehost)

COBOL itself is rehosted: the same source, compiled by a modern open-source COBOL compiler (GnuCOBOL) on commodity cloud instances, with JCL batch orchestration re-expressed as cloud-native schedulers. This is the fastest, lowest-risk path - no logic translation at all - and it buys immediate relief from mainframe hardware and licensing economics while preserving the option to modernize function by function later. It is the right path for logic-dense programs with low data volume, unusual language features, or thin test coverage, where translation risk exceeds translation benefit. The equivalence gate still applies: rehosted COBOL must produce byte-identical output to the original.

### Path C: The AI-enabled solution (the middle way)

AI does not replace the two paths above - it accelerates both and governs them. In our delivery:

- **AI reads the estate** - ingestion of COBOL source, copybooks, JCL, data dictionaries, and runbooks; cross-reference and dependency mapping; dead-code detection; business-rule extraction into human-reviewable specifications.
- **AI drafts, humans decide** - first-pass translations (COBOL to modern languages), first-pass test suites derived from the extracted rules, and first-pass data mappings. Every AI artifact is reviewed and signed by engineers, and every claim it makes is then verified by the equivalence gate, which is deliberately NOT AI - it is a deterministic byte comparison. AI proposes; the gate disposes.
- **AI assists in production** - anomaly detection during dual-run, reconciliation triage, and runbook generation for the new platform.

The decision matrix we apply per function:

| Workload characteristic | Path |
|-------------------------|------|
| High data volume, aggregations, ETL-heavy | A - modernize, GPU/CPU tier |
| Logic-dense, low volume, thin test coverage | B - rehost COBOL on cloud |
| High change frequency, needs modern tooling | A - modernize |
| Regulatory freeze, no tolerance for change | B - rehost |
| Medium complexity, wants staged transition | C - AI-assisted, then A or B per function |

## 3. The governed migration cycle

Every engagement runs the same seven phases. Governance artifacts are produced at each phase and are contractually inspectable by the client.

1. **Grab.** We take custody of source code, copybooks, JCL/PROC libraries, data dictionaries, scheduler catalogs, and SMF-style run statistics - with provenance and access controls. Nothing is re-keyed.
2. **Review.** AI-assisted static analysis and cross-reference produce the function inventory: what each program does, what it reads and writes, what depends on it, what is dead. This is the migration specification - the contract for everything that follows.
3. **Rebuild.** Each function is re-implemented (Path A), recompiled (Path B), or AI-drafted then engineered (Path C) onto its assigned tier. Business logic is preserved one-to-one in semantics; only the data-access layer changes.
4. **Deploy.** Rebuilt functions become containerized jobs and services on the target platform, orchestrated by a DAG scheduler that preserves batch windows, dependencies, and restart semantics.
5. **Migrate data.** DB2 tables, VSAM files, and QSAM/GDG datasets move to canonical target formats with fixed-point decimal semantics preserved exactly, at the precision the schema declares - no float drift.
6. **Test.** The equivalence gate: every rebuilt function must reproduce its legacy reference output byte-for-byte across the full test corpus, including edge cases, totals, and ordering. This gate runs continuously - in CI, in the benchmark harness, and during dual-run - so divergence cannot silently accumulate. Any mismatch fails the job automatically.
7. **Roll over.** Dual-run shadow operations against production, reconciliation via the gate, then cutover function by function. Risk is bounded to one function at a time, and every step is reversible until the decommission order is signed.

## 4. The equivalence guarantee: why trust is a measurement

Vendors market automation percentages - '99.7% automated'. An automation rate is a claim about effort. It says nothing about whether the remaining 0.3% - or for that matter the other 99.7% - behaves correctly. The industry's failure stories are stories of systems validated by inspection.

Bylazora's acceptance criterion is mechanical:

- **Exact arithmetic.** COBOL business math is fixed-point. We carry money as integer cents end-to-end, so the new implementation is provably exact, not approximately close.
- **Byte-exact outputs.** The migrated function must produce output identical byte-for-byte to the legacy reference - field values, totals, ordering, formatting. One differing byte is a failure with a diff attached.
- **Continuous enforcement.** The equivalence check is wired into the toolchain and re-run on every change, every scale-up, every re-run. Equivalence is not a one-time acceptance ceremony; it is a standing property of the delivered system.

This is the property no major migration vendor publishes, and it is the foundation of every other claim in this paper.

## 5. Performance by evidence: the benchmark that decides

Bylazora benchmarks before it recommends. Our reference implementation - a representative COBOL batch program performing per-account aggregation over transaction files - runs on the shipped Rust engine across three tiers (CPU, wgpu GPU, CUDA container) and is benchmarked at four scales - 1M to 1B rows - three cold-cache repetitions each, every output byte-identical to the COBOL reference:

| Scale | COBOL (baseline) | CPU tier (Rust) | GPU tier (wgpu) | GPU tier (CUDA container) |
|-------|-----------------|-----------------|-----------------|---------------------------|
| 1M    | 1.00x           | 26.3x           | 39.5x           | 39.5x                      |
| 10M   | 1.00x           | 48.5x           | 48.5x           | 44.5x                      |
| 100M  | 1.00x           | 25.5x           | 36.1x           | 60.4x                      |
| 1B    | 1.00x           | 28.1x           | 61.3x           | 68.8x                      |

Three honest conclusions. GPU acceleration is transformative at scale - the cudarc CUDA tier holds 68.8x at 1B rows on a single consumer RTX 3080 (8,704 CUDA cores) and the vendor-neutral wgpu tier 61.3x on the same card, consistent with independent industry data (NVIDIA reports cuDF accelerating pandas workloads by up to 150x on GB-scale ETL; AWS reports up to 3.7x for GPU-accelerated Spark; TPC-H GPU query engines report 7.5x and higher). At small scale the RAPIDS container the cudarc tier replaces lost - 0.2x at 1M rows, because transfer and launch overheads exceeded compute savings; those numbers are provenance now. And the read path matters as much as the compute: the same 8-thread parallel chunked reader that holds the CPU tier's 28.1x at 1B rows now drives both GPU tiers, lifting the 1B row from 50.5s to 9.8s (wgpu) and 8.75s (cudarc). The crossover remains the router: benchmark, then choose.

We publish the crossover because it is our policy engine: workloads whose volumes justify acceleration get GPUs; everything else gets the appropriate tier. Clients never pay for acceleration that does not accelerate.

## 6. The target architecture

**Data.** Canonical Parquet datasets on object storage with fixed-point decimal semantics (COBOL COMP-3 maps to exact decimal/integer types - no float drift); keyed stores for VSAM-class access patterns; versioned objects for generation data groups. The mainframe continues to run during transition; change-data capture and bulk unloads keep the new platform current until cutover.

**Functions.** Every program becomes a containerized job or service. JCL steps become scheduler DAG nodes with preserved windows and dependencies. Online transaction programs become horizontally-scalable stateless services behind load balancers (the cloud-native shape of a CICS region).

**Compute tiers.** (1) General-purpose instances running rehosted COBOL; (2) CPU-optimized instances running the Rust CPU tier; (3) dedicated-GPU instances (NVIDIA L4/L40S/A100/H100 classes) running the wgpu tier on any GPU, or the RAPIDS container path.

**AI layer.** Discovery, rule extraction, test generation, translation assistance, and operational anomaly detection - with every AI artifact gated by deterministic validation before it can affect production.

## 7. Scaling: from steady state to tax time

Peak demand is the test every migrated platform must pass. A revenue agency at tax time sees online transaction volumes multiply and batch processing (returns, refunds, assessments) spike simultaneously. Bylazora designs for the peak, then scales down to the steady state, in both dimensions:

**Online demand (transactions).** Migrated online workloads are stateless, horizontally-scalable services behind load balancers. Auto-scaling groups scale the service tier on request rate and latency SLOs; the data tier scales independently (read replicas, provisioned throughput). Capacity is tested against replay of captured production peak traffic - not synthetic loads - so the SLO evidence predates go-live.

**Batch demand (processing).** The batch estate is elastic by construction: the DAG scheduler drives container fleets that expand with the work queue. For a defined peak window, GPU and CPU node pools are pre-warmed and scaled out before the season opens; spot fleets add cost-efficient burst capacity for checkpointable jobs, while reserved/capacity-block instances guarantee the floor. Sharded workloads add shards; multi-GPU nodes add GPUs; the merge layer stays cheap because reductions are O(#accounts), not O(#rows).

**The capacity contract.** For every peak season we deliver: a demand forecast derived from historical run statistics; a capacity plan mapping the forecast to instance counts per tier; a pre-warm runbook with exact timings; load and throughput SLOs with alerting; and a cost model for the peak so the client knows the seasonal bill before the season starts. Peak days become a planned, budgeted, rehearsed event - not a surprise.

**Scale-out beyond one GPU.** Workloads shard by business key - account, customer, policy - never by time, preserving inter-row semantics per shard. On our benchmark box, one NVIDIA RTX 3080 processed 100M rows of the reference workload in 6.6 seconds and 1B rows in 68 seconds; L4/L40S-class deployment instances run the same workload class in seconds. Estates that exceed one GPU's memory are partitioned across GPUs and nodes with GPU-native dataframe engines (Dask-cuDF, Spark-RAPIDS), with near-linear scaling because the reduction step is small. Sizing is computed by benchmarking the client's own workloads with the harness - the numbers set the instance counts, not a slide deck.

## 8. Indicative economics

**Platform cost.** A worked example: nightly batch of 100M transactions, 3 hours daily processing, one L40S-class GPU instance (18,176 CUDA cores) plus one CPU instance, 2 TB object storage, 200 GB monthly egress - roughly USD 380/month on AWS on-demand, before spot and commitment discounts. Against a conservative USD 3,000-8,000/month mainframe allocation that is an 87-95% reduction; published industry outcomes are consistent (30-50% three-year TCO reduction with ~22-month median payback; named cases report up to 70-90%).

**Engagement model.** Fixed-fee and value-anchored: discovery, per-job migration tiers, and optional ongoing support are quoted against the assessed function inventory - not hours - and shared at proposal stage. Pricing detail is deliberately kept out of public documents so every quote reflects the estate in front of us. Acceptance criteria are contractual: byte-exact equivalence and the benchmark evidence.

## 9. Risk, governance, and compliance

- **Dual-run shadowing.** The legacy and target run in parallel through at least one full business cycle; the equivalence gate reconciles every output. Cutover is per function and reversible.
- **Rollback.** Every migration ships a rollback path: the legacy system remains operable until decommission sign-off.
- **Data governance.** Fixed-point fidelity, lineage from source extraction, access controls, and retention policies mirror the client's existing regimes; the migration itself is auditable end-to-end.
- **Security.** Extraction runs over approved channels; the target platform inherits cloud-native controls (encryption at rest and in transit, IAM, VPC isolation) and the client's compliance frameworks.
- **The AI boundary.** AI assists analysis, drafting, and operations - it never silently alters business logic. Every AI-generated artifact passes human review and the deterministic equivalence gate before it can affect a production path.

## 10. Engagement model and timeline

- **Discovery** (2-6 weeks per workload cluster): function inventory with hot-path ranking, data-interface catalog, mainframe-replication test environment, and a fixed-fee migration proposal. No production change occurs during discovery.
- **Migration** (per job tier, typically weeks per job): the seven-phase cycle, ending in equivalence-proven acceptance artifacts.
- **Shadow run and cutover** (at least one full business cycle): dual-run, reconciliation, per-function cutover.
- **Steady state** (optional): your team runs the target platform on the open engine. Continuous equivalence monitoring, the seasonal peak plan, and SLO-managed operation are available from us if you want them.

## 11. How fast: the six-month program

Bylazora targets end-to-end delivery of a representative estate within six months. The calendar is fixed; the scope scales through parallel migration waves, not longer timelines:

| Program phase | Weeks | What happens |
|---------------|-------|--------------|
| Grab + Review (discovery) | 1-4 | Code and documentation intake; AI-assisted inventory; hot-path ranking; data-interface catalog; mainframe-replication test environment |
| Rebuild + Deploy + Migrate data | 5-16 | Migration waves - many functions in parallel, each re-implemented or rehosted on its tier, each equivalence-proven before it proceeds |
| Test + shadow run | 17-20 | Dual-run against production through a full business cycle; continuous reconciliation via the equivalence gate |
| Roll over + decommission | 21-24 | Per-function cutover; parallel-run sign-off; decommissioning; handover to your team |
| Steady state | 25+ | Your team runs it on the engine: continuous equivalence monitoring and seasonal peak plans. Optional SLO-managed operation from us. |

Three properties make a six-month program credible where traditional migrations run years: parallel waves are safe because every function carries its own byte-exact proof; the mainframe keeps running throughout, so the business never cuts over blind; and each phase has a hard governance artifact, so slippage is visible in week one, not month eighteen.

## 12. Why Bylazora

- **Proof, not percentages.** Byte-exact equivalence is our acceptance criterion, published and contractual.
- **All three paths.** Move off COBOL, rehost it to the cloud, or take the AI-enabled route - chosen per workload by evidence.
- **Performance honesty.** We publish the crossover data, including where GPU acceleration does not pay. Clients trust the recommendation because the numbers are visible.
- **Peak-proven.** The capacity contract means tax time, month-end, and season peaks are rehearsed, budgeted events.
- **Priced against outcome.** Fixed-fee tiers tied to the assessed inventory and the equivalence commitment, not to hours spent.

---

*Figures cited from public sources (IBM, MarketsandMarkets, Gartner, Barclays, ITIC, Deloitte, NVIDIA, AWS, and others, 2023-2026) and from Bylazora's own benchmark measurements. Savings figures are ranges or named cases; Bylazora does not promise a percentage - it promises a measurement.*

**Contact: bylazora.com**
## 13. Further information

**Bylazora | bylazora.com**

For further information, the full benchmark evidence, and to book a discovery: https://bylazora.com

Email: hello@bylazora.com

