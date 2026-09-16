// SPDX-License-Identifier: AGPL-3.0-or-later
// bylazora-core library: the migration engine and its C ABI.
//
// The crate exposes the engine tiers (cpu, gpu, cuda), the byte-exact
// validator, the offline licence gate, and a C ABI (capi) so enterprises can
// integrate the engine from C, C++, or any FFI. No component performs network
// I/O: the engine runs air-gapped, and the licence check is a pure function of
// the key, the row count, and the clock.
pub mod capi;
pub mod copybook;
pub mod cpu;
#[cfg(feature = "cuda")]
pub mod cuda;
pub mod db2;
pub mod gpu;
pub mod key;
pub mod migrate;
pub mod validator;
