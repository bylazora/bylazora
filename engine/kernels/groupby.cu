// SPDX-License-Identifier: AGPL-3.0-or-later
// CUDA tier kernel: per-row atomic groupby, mirror of the WGSL kernel.
// Amounts are unsigned cents; per-account totals fit u32 for the reference
// workload (same contract the wgpu tier already certifies byte-exact).
extern "C" __global__ void groupby(const int* __restrict__ acct,
                                   const unsigned* __restrict__ amt,
                                   const unsigned* __restrict__ is_dep,
                                   unsigned* deposits,
                                   unsigned* withdrawals,
                                   unsigned* counts,
                                   long n) {
    long i = blockIdx.x * (long)blockDim.x + threadIdx.x;
    if (i >= n) return;
    int a = acct[i];
    atomicAdd(&deposits[a], is_dep[i] * amt[i]);
    atomicAdd(&withdrawals[a], (1u - is_dep[i]) * amt[i]);
    atomicAdd(&counts[a], 1u);
}
