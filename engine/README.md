# bylazora-core

**Proof, not percentages.** The byte-exact COBOL mainframe migration engine: one
Rust binary runs the equivalence gate and the production runtime across three
compute tiers - CPU, any GPU through vendor-neutral wgpu, and CUDA - and speaks
JSON-RPC 2.0 over stdio as an MCP server, so a coding agent can drive it.

A migrated workload's outputs must equal the legacy COBOL reference byte for
byte, or the gate fails it: one differing byte is a failure with a diff. Money
stays integer cents and is never a float.

- AGPL-3.0-or-later for the engine, Apache-2.0 for the copybook and DB2 parsers.
- Homepage: https://bylazora.com
- Repository: https://github.com/bylazora/bylazora
- Evidence annex: https://bylazora.com/evidence.html
