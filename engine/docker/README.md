# bylazora-core GPU container image

Docker image for the bylazora-core migration engine (core-rs, version 0.3.1),
built around its vendor-neutral wgpu GPU tier.

The engine binary is statically linked except glibc, so the image adds only
what the GPU tier needs at run time:

- the Vulkan loader (libvulkan1), loaded by wgpu on demand
- the Mesa Vulkan ICDs (mesa-vulkan-drivers), which provide the hardware
  drivers and lavapipe (llvmpipe), the software Vulkan fallback

The base is debian:bookworm-slim, pinned to a content digest so every build
starts from the same layer.

## Build

Run from this directory (core-rs/docker):

```
docker build -t bylazora-core:0.3.1 .
```

The build context is this directory, so it holds only the Dockerfile, this
README, and the prebuilt binary bylazora-linux-x86_64. That binary is a copy of
tools/assets/bylazora-linux-x86_64 at version 0.3.1. Refresh it with:

```
cp ../../tools/assets/bylazora-linux-x86_64 bylazora-linux-x86_64
```

## Verify the image

```
docker run --rm bylazora-core:0.3.1 --version
```

Expected output: bylazora-core 0.3.1

## Run a bench

The bench subcommand reads balances.csv and transactions.csv from an input
directory and writes final_balances.csv and summary_report.csv to an output
directory. Mount one data volume for both:

```
docker run --rm \
  -v "$(pwd)/data:/data" \
  bylazora-core:0.3.1 \
  bench /data/in /data/out --backend gpu
```

Point /data/in and /data/out at directories inside the mounted volume. The
container runs as uid 10001 (the bylazora user), so the mounted directory must
be writable by that uid, for example chown 10001 data.

The --backend flag is cpu (default) or gpu. Use --adapter N to pick a specific
wgpu adapter; the default is adapter 0.

## GPU access

Pass the host GPU through with the NVIDIA Container Toolkit:

```
docker run --rm --gpus all \
  -v "$(pwd)/data:/data" \
  bylazora-core:0.3.1 \
  bench /data/in /data/out --backend gpu
```

--gpus all requires the NVIDIA Container Toolkit on the host. On an AMD or
Intel Vulkan host, or on a host with no GPU at all, omit --gpus and the engine
falls back to lavapipe, the Mesa software Vulkan driver:

```
docker run --rm \
  -v "$(pwd)/data:/data" \
  bylazora-core:0.3.1 \
  bench /data/in /data/out --backend gpu
```

The first line of GPU output names the selected adapter, for example
adapter[0]: NVIDIA GeForce RTX 3080 (Vulkan) for hardware, or
adapter[0]: llvmpipe (LLVM ...) (Vulkan) for the software fallback.

## Security posture

The engine performs no network I/O and no telemetry. The licence check is a
pure function of the key, the row count, and the clock, so the engine runs
air-gapped. The container runs as an unprivileged user (uid 10001) with no
login shell, and the image installs no network services. The only host
resources the container needs are the data volume mounts and, when requested,
the GPU.

The free Developer tier allows up to 10,000,000 rows per job without a key.
Above that, install a production key with bylazora-core licence set <key> or
the BYLAZORA_KEY environment variable.

## Reproducibility

The base image digest and the prebuilt binary are both pinned. To rebuild on a
different base, update the FROM ... @sha256:... line. To refresh the binary,
copy a new bylazora-linux-x86_64 from tools/assets/.
