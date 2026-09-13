/*
 * bylazora.h - C ABI for the Bylazora migration engine.
 *
 * Integrate the engine from C, C++, or any FFI. All functions are panic-free
 * across the ABI boundary, and none performs network I/O: the engine runs
 * air-gapped, and the licence check is a pure function of the key, the row
 * count, and the clock. Paths are UTF-8 and are passed as ordinary
 * NUL-terminated char pointers.
 */
#ifndef BYLAZORA_H
#define BYLAZORA_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Return codes shared by every function. */
#define BYLAZORA_OK 0       /* success, or byte-identical for validate */
#define BYLAZORA_MISMATCH 1 /* validate only: the outputs differ */
#define BYLAZORA_ERROR (-1) /* error; errbuf (when given) holds a message */

/*
 * Return the version of this interface. The pointer is static and
 * NUL-terminated and must not be freed by the caller.
 */
const char *bylazora_version(void);

/*
 * Byte-compare two output directories. Returns BYLAZORA_OK (0) when the
 * reference and other directory hold identical final_balances.csv and
 * summary_report.csv files, BYLAZORA_MISMATCH (1) when they differ, and
 * BYLAZORA_ERROR (-1) on error (for example a missing or unreadable file).
 *
 * On error only, errbuf is filled with a NUL-terminated message truncated to
 * errbuf_len - 1 bytes. errbuf may be NULL; errbuf_len 0 is a no-op.
 */
int32_t bylazora_validate(const char *ref_dir, const char *other_dir,
                          char *errbuf, size_t errbuf_len);

/*
 * Run a tier and write byte-exact outputs into output_dir. backend selects the
 * tier: "cpu" (the default, used when backend is NULL or empty), "gpu"
 * (vendor-neutral wgpu, adapter index 0), or "cuda" (NVIDIA device 0). Returns
 * BYLAZORA_OK (0) on success and BYLAZORA_ERROR (-1) on error with errbuf
 * filled as described above.
 */
int32_t bylazora_bench(const char *backend, const char *input_dir,
                       const char *output_dir, char *errbuf, size_t errbuf_len);

/*
 * Check the installed licence key. Returns BYLAZORA_OK (0) when a valid,
 * unexpired key is installed and BYLAZORA_ERROR (-1) otherwise with errbuf
 * filled as described above. The licence signature is never printed and never
 * returned in errbuf.
 */
int32_t bylazora_licence_check(char *errbuf, size_t errbuf_len);

#ifdef __cplusplus
}
#endif

#endif /* BYLAZORA_H */
