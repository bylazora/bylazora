# Bylazora

**The migration you can prove.**

Bylazora is a byte-exact migration engine: a migrated workload's outputs must
equal the legacy COBOL reference byte for byte, or the gate fails it. One
Rust binary runs the verification gate and the production runtime across three
tiers - CPU, vendor-neutral wgpu GPU, and cudarc CUDA - with fixed-point
decimal fidelity (money as integer cents, never float). Measured byte-exact at
one billion rows: 28.1x (CPU), 61.3x (wgpu), 68.8x (cudarc) against the COBOL
baseline. Proof, not percentages.

## What is in this repository

- engine/ - the engine source (AGPL-3.0-or-later): gate, three tiers, C ABI, MCP
- parsers/ - the copybook parser and DB2 importer (Apache-2.0)
- sample/ - the sample migration: a COBOL batch program and its migrated engine,
  with the byte-exact gate as the CI regression test
- docs/ - the white papers and the evidence annex (every public claim traces to
  a recorded run)
- kernels/ - the WGSL and CUDA kernel sources
- include/ - the C ABI header

## Licences

The engine is AGPL-3.0-or-later: use it, modify it, run it - and if you offer a
modified version over a network, share it. The parsers are Apache-2.0 so
mainframe teams can embed them freely. A commercial licence (for AGPL-averse
estates or closed embedding, with support) and the managed run are sold by
Bylazora - see bylazora.com.

## Quick start

Build (Rust, wgpu, CUDA optional): cargo build --release
Run the sample end to end: sample/run_all.sh

## Community

Discussions and issues are welcome. See CONTRIBUTING.md. Bylazora is an
independent project; the Open Mainframe Project is the target governance home.
