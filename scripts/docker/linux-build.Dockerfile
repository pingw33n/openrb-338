# The Linux build environment of scripts/build.sh (task-108): Ubuntu 22.04 (glibc 2.35, so the
# binaries run on distros from 2022 on) with the C toolchain and the ALSA headers cpal needs.
# `file` and squashfs-tools are there for appimagetool (scripts/package-appimage.sh packs in
# this image off Linux; scripts/appimage-pack.sh). libxkbcommon-x11-0 (with libxkbcommon0 and
# libxcb-xkb1) is the source of the three libraries the AppImage bundles (task-166).
# The Rust toolchain is not baked in: rustup installs the one pinned in rust-toolchain.toml
# on the first cargo call, into a volume that scripts/build.sh keeps.
#
# The base image is pinned by the digest of the multi-arch index of ubuntu:22.04 (looked up
# 2026-10-28); bump it on purpose. The rustup installer fetched below is NOT checksummed
# (curl | sh over TLS), and the apt packages are the distro's current ones.
FROM ubuntu:22.04@sha256:e0f80c829509ef4df9e76bd8944eb7b8ec25cbb379b112a7f9448b2c77a77c9b
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential pkg-config libasound2-dev cmake curl ca-certificates git file squashfs-tools \
        libxkbcommon-x11-0 patchelf \
    && rm -rf /var/lib/apt/lists/*
ENV RUSTUP_HOME=/rustup CARGO_HOME=/cargo PATH=/cargo/bin:$PATH
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain none --profile minimal
