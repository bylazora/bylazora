// SPDX-License-Identifier: AGPL-3.0-or-later
// CUDA tier kernel: per-row atomic groupby, mirror of the WGSL kernel.
// Amounts are unsigned cents and still fit u32 per transaction. Per-account
// running totals do not: CUDA has native 64-bit atomicAdd (unsigned long
// long, supported since compute capability 2.0, well below this kernel's
// sm_80 target), so deposits and withdrawals accumulate in 64-bit directly,
// unlike the WGSL tier which has to emulate it with a pair of u32 atomics.
extern "C" __global__ void groupby(const int* __restrict__ acct,
                                   const unsigned* __restrict__ amt,
                                   const unsigned* __restrict__ is_dep,
                                   unsigned long long* deposits,
                                   unsigned long long* withdrawals,
                                   unsigned* counts,
                                   long n) {
    long i = blockIdx.x * (long)blockDim.x + threadIdx.x;
    if (i >= n) return;
    int a = acct[i];
    atomicAdd(&deposits[a], (unsigned long long)(is_dep[i] * amt[i]));
    atomicAdd(&withdrawals[a], (unsigned long long)((1u - is_dep[i]) * amt[i]));
    atomicAdd(&counts[a], 1u);
}
