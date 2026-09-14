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

- engine/ - the engine source (AGPL-3.0-or-later): gate, three tiers, the C ABI
  (engine/include/bylazora.h), the MCP server, and Cargo.lock pinning the
  dependency set the shipped binaries were built from
- parsers/ - the copybook parser and DB2 importer, mirrored out of engine/src
  under Apache-2.0 so mainframe teams can embed them without the AGPL terms
- judge/ - the gate-as-judge harness: one verdict from a byte-exact comparison,
  usable standalone (`python judge.py --ref-dir ... --candidate-dir ...`) or
  as the scoring step in an agent's migration loop
- agents/ - rule packs for AI coding agents working against this standard
  (CLAUDE.md and friends): the invariants an agent must respect, most of all
  that no tool parameter can mark a run proven
- sample/ - the sample migration: a COBOL batch program and its migrated engine,
  with the byte-exact gate as the CI regression test
- docs/ - the white papers and the evidence annex (every public claim traces to
  a recorded run)

Inside engine/: kernels/ holds the CUDA kernel source (groupby.cu, compiled to
groupby.ptx); the wgpu compute shader is inline WGSL in src/gpu.rs, not a
separate file. include/ holds the C ABI header (bylazora.h) for the four
exported symbols (bylazora_version, bylazora_validate, bylazora_bench,
bylazora_licence_check).

## Licences

The engine is AGPL-3.0-or-later: use it, modify it, run it - and if you offer a
modified version over a network, share it. The parsers are Apache-2.0 so
mainframe teams can embed them freely. A commercial licence (for AGPL-averse
estates or closed embedding, with support) and the managed run are sold by
Bylazora - see bylazora.com.

## Quick start

```
cd engine
cargo build --release              # CPU and wgpu tiers; no CUDA toolkit needed
cargo build --release --features cuda   # adds the CUDA tier (needs nvcc)
bash ../sample/run_all.sh          # the byte-exact gate, end to end
```

The prebuilt Linux x86_64 binary in sample/engine/ needs glibc 2.34 or newer
(Ubuntu 22.04 / Debian 12 / RHEL 9 and later); building from source has no
such floor.

## The MCP server

`bylazora-core mcp` speaks JSON-RPC 2.0 over stdio and exposes three tools:
validate, bench, and copybook_parse. Point an MCP-capable agent at it with:

```json
{
  "mcpServers": {
    "bylazora": { "command": "/path/to/bylazora-core", "args": ["mcp"] }
  }
}
```

The gate rule holds through this interface exactly as it does on the CLI: the
validate tool is the only thing that can report IDENTICAL, and no argument to
any tool can mark a run proven on its own say-so.

## Community

Discussions and issues are welcome. See CONTRIBUTING.md. Bylazora is an
independent project; the Open Mainframe Project is the target governance home.
