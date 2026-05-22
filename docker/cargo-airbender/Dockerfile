FROM ubuntu:22.04 AS rust-base

ARG DEBIAN_FRONTEND=noninteractive
ARG RUST_TOOLCHAIN=nightly-2026-02-10

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:${PATH} \
    CARGO_TERM_COLOR=always

# Keep the image useful as a cargo-airbender environment, not just as a binary
# carrier. Guest build flows need Cargo, rust-src, cargo-binutils, and linker
# tooling even when the CLI is built without CUDA/GPU proving support.
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        clang \
        cmake \
        curl \
        git \
        libssl-dev \
        lld \
        pkg-config \
        wget && \
    rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --no-modify-path --default-toolchain "${RUST_TOOLCHAIN}"

RUN rustup component add llvm-tools-preview rust-src && \
    cargo install cargo-binutils --locked

FROM rust-base AS builder

WORKDIR /usr/src/airbender-platform

COPY . .

RUN cargo build \
        -p cargo-airbender \
        --release \
        --locked \
        --no-default-features \
        --target x86_64-unknown-linux-gnu

FROM rust-base

COPY --from=builder /usr/src/airbender-platform/target/x86_64-unknown-linux-gnu/release/cargo-airbender /usr/local/bin/cargo-airbender

WORKDIR /workspace

ENTRYPOINT ["cargo-airbender"]
CMD ["--help"]
