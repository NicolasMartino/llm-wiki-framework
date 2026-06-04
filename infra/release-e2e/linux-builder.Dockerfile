FROM rust:1-bookworm

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        clang \
        cmake \
        libclang-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*
