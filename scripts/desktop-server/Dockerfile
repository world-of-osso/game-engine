# syntax=docker/dockerfile:1
FROM rust:1.98.1-bookworm AS build
RUN apt-get update && apt-get install -y --no-install-recommends clang libclang-dev mold cmake python3 pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /src/game-server
ARG TARGET_CACHE
ARG SERVER_CACHE
ARG JOBS=8
ENV CARGO_TARGET_DIR=/src/game-server/target \
    CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=clang \
    RUSTFLAGS="-C link-arg=-fuse-ld=mold -C link-arg=-Wl,--thread-count=4"
COPY . /src/
RUN --mount=type=cache,id=server-registry-${SERVER_CACHE},target=/usr/local/cargo/registry \
    --mount=type=cache,id=server-git-${SERVER_CACHE},target=/usr/local/cargo/git \
    --mount=type=cache,id=${TARGET_CACHE},target=/src/game-server/target,sharing=locked \
    python3 /src/refresh-source-mtimes.py /src && \
    cargo build --locked -p server --bin game-server --bin game-server-admin -p client --bin game-cli -j "$JOBS" && \
    mkdir -p /out && \
    gzip -1 -c target/debug/game-server > /out/game-server.gz && \
    gzip -1 -c target/debug/game-server-admin > /out/game-server-admin.gz && \
    gzip -1 -c target/debug/game-cli > /out/game-cli.gz
FROM scratch AS artifact
COPY --from=build /out/ /
