FROM rust:1-bookworm

ARG CARGO_DIST_VERSION=0.28.0

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        clang \
        cmake \
        curl \
        libclang-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-dist --version "${CARGO_DIST_VERSION}" --locked --root /usr/local
