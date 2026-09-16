# bylazora-core GPU container image

Docker image for the bylazora-core migration engine (version 0.5.0), built
around its vendor-neutral wgpu GPU tier.

The engine binary is statically linked except glibc, so the image adds only
what the GPU tier needs at run time:

- the Vulkan loader (libvulkan1), loaded by wgpu on demand
- the Mesa Vulkan ICDs (mesa-vulkan-drivers), which provide the hardware
  drivers and lavapipe (llvmpipe), the software Vulkan fallback

The base is debian:bookworm-slim, pinned to a content digest so every build
starts from the same layer.

## Build

Build the engine, then the image, both from the repository root:

```
cargo build --release --manifest-path engine/Cargo.toml
docker build -t bylazora-core:0.5.0 -f engine/docker/Dockerfile .
```

The Dockerfile COPYs the binary from engine/target/release/, the normal
engine build output, so there is no second copy of the binary to keep in
sync.

## Verify the image

```
docker run --rm bylazora-core:0.5.0 --version
```

Expected output: bylazora-core 0.5.0

## Run a bench

The bench subcommand reads balances.csv and transactions.csv from an input
directory and writes final_balances.csv and summary_report.csv to an
output directory. Mount one data volume for both:

```
docker run --rm \
  -v "$(pwd)/data:/data" \
  bylazora-core:0.5.0 \
  bench /data/in /data/out --backend gpu
```

Point /data/in and /data/out at directories inside the mounted volume. The
container runs as uid 10001 (the bylazora user), so the mounted directory
must be writable by that uid, for example chown 10001 data.

The --backend flag is cpu (default), gpu, or cuda. Use --adapter N to pick
a specific wgpu adapter for the gpu backend; the default is adapter 0. This
image is built without the cuda cargo feature (see engine/Cargo.toml), so
the cuda backend is unavailable inside the container; use the gpu backend,
which runs on any Vulkan adapter including NVIDIA hardware.

## GPU access

Pass the host GPU through with the NVIDIA Container Toolkit:

```
docker run --rm --gpus all \
  -v "$(pwd)/data:/data" \
  bylazora-core:0.5.0 \
  bench /data/in /data/out --backend gpu
```

--gpus all requires the NVIDIA Container Toolkit on the host. On an AMD or
Intel Vulkan host, or on a host with no GPU at all, omit --gpus and the
engine falls back to lavapipe, the Mesa software Vulkan driver:

```
docker run --rm \
  -v "$(pwd)/data:/data" \
  bylazora-core:0.5.0 \
  bench /data/in /data/out --backend gpu
```

The first line of GPU output names the selected adapter, for example
adapter[0]: NVIDIA GeForce RTX 3080 (Vulkan) for hardware, or
adapter[0]: llvmpipe (LLVM ...) (Vulkan) for the software fallback. That
line is written to stderr, not stdout, so it does not interfere with
anything reading the container's stdout.

## Security posture

The engine performs no network I/O and no telemetry. A licence key, when
installed, is resolved offline as a licensee record for the run record and
the licensee's own audit; nothing about it gates a run. The engine is free
and unlimited under AGPL-3.0-or-later. The container runs as an
unprivileged user (uid 10001) with no login shell, and the image installs
no network services. The only host resources the container needs are the
data volume mounts and, when requested, the GPU.

## Reproducibility

The base image digest is pinned; update the FROM ... @sha256:... line to
rebuild on a different base. The binary is the normal engine release build
(cargo build --release --manifest-path engine/Cargo.toml, without
--features cuda to match this image).
