# bylazora-core GPU container image.
#
# bylazora-core is a Rust binary (core-rs, version 0.4.1) with a vendor-neutral
# wgpu GPU tier that runs on any Vulkan adapter. The binary is statically linked
# except glibc, so the image only needs glibc, the Vulkan loader, and the Mesa
# Vulkan ICDs.
#
# Build from the repository root, not this directory, so the COPY below can
# reach the one built binary the project ships: see README.md.
#   docker build -t bylazora-core:0.4.1 -f core-rs/docker/Dockerfile .
#
# Base: Debian 12 (bookworm) slim, pinned to a content digest for reproducible
# builds. Its glibc 2.36 satisfies the binary's GLIBC_2.34 requirement.
FROM debian:bookworm-slim@sha256:88200866dfff7ea7f5cbcb6ec7c8a701889efe6fe859fe64d6990e4b07ea4171

# The Vulkan loader (libvulkan.so.1, loaded by wgpu at run time) and the Mesa
# Vulkan ICDs. Mesa provides lavapipe (llvmpipe), the software Vulkan driver
# used as the no-GPU fallback, plus the AMD and Intel hardware ICDs. On an
# NVIDIA host the NVIDIA ICD and driver are injected by Docker at run time via
# --gpus all. --no-install-recommends skips recommended packages so only the
# Vulkan runtime is added. No windowing system is required: the engine is
# headless compute only.
RUN export DEBIAN_FRONTEND=noninteractive \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libvulkan1 \
        mesa-vulkan-drivers \
    && rm -rf /var/lib/apt/lists/*

# The prebuilt engine binary. See README.md for its source of truth. Path is
# relative to the build context (the repo root; see the build command above).
COPY deliverables/sample-migration/engine/bylazora-core /usr/local/bin/bylazora-core

# Run as an unprivileged user. The engine performs no network I/O and writes
# only to data volumes the operator bind-mounts.
RUN useradd --create-home --uid 10001 --shell /usr/sbin/nologin bylazora \
    && chmod 0755 /usr/local/bin/bylazora-core \
    && mkdir -p /data \
    && chown bylazora:bylazora /data

USER bylazora
WORKDIR /data

ENTRYPOINT ["/usr/local/bin/bylazora-core"]
