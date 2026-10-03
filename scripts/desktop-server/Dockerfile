# syntax=docker/dockerfile:1
FROM rust:1.98.1-bookworm AS build
RUN apt-get update && apt-get install -y --no-install-recommends clang libclang-dev mold cmake python3 pkg-config && rm -rf /var/lib/apt/lists/*
ARG BUILD_ROOT
ARG BUILD_PARENT
ARG TARGET_CACHE
ARG SERVER_CACHE
ARG JOBS=8
ARG RELEASE=false
ARG BINARY
ENV CARGO_TARGET_DIR=/build-target \
    CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=clang \
    RUSTFLAGS="-C link-arg=-fuse-ld=mold -C link-arg=-Wl,--thread-count=4"
COPY game-server/ ${BUILD_ROOT}/
COPY shared-protocol/ ${BUILD_PARENT}/shared-protocol/
COPY refresh-source-mtimes.py ${BUILD_PARENT}/refresh-source-mtimes.py
WORKDIR ${BUILD_ROOT}
# Only these two source trees may be refreshed, even if BUILD_PARENT contains image files.
RUN --mount=type=cache,id=server-registry-${SERVER_CACHE},target=/usr/local/cargo/registry \
    --mount=type=cache,id=server-git-${SERVER_CACHE},target=/usr/local/cargo/git \
    --mount=type=cache,id=${TARGET_CACHE},target=/build-target,sharing=locked \
    python3 -c 'import runpy, sys; from pathlib import Path; from types import SimpleNamespace; refresh = runpy.run_path(sys.argv[1])["refresh_sources"]; refresh(SimpleNamespace(iterdir=lambda: [Path(path) for path in sys.argv[2:]]))' \
        "$BUILD_PARENT/refresh-source-mtimes.py" "$BUILD_ROOT" "$BUILD_PARENT/shared-protocol" && \
    case "$BINARY" in \
        "") binaries="game-server game-server-admin game-cli"; set -- -p server --bin game-server --bin game-server-admin -p client --bin game-cli ;; \
        game-server|game-server-admin) binaries="$BINARY"; set -- -p server --bin "$BINARY" ;; \
        game-cli) binaries="$BINARY"; set -- -p client --bin "$BINARY" ;; \
        *) echo "unsupported server binary: $BINARY" >&2; exit 1 ;; \
    esac && \
    profile=debug && \
    if [ "$RELEASE" = true ]; then profile=release; set -- "$@" --release; fi && \
    cargo build --locked -j "$JOBS" "$@" && \
    mkdir -p /out && \
    for binary in $binaries; do \
        gzip -1 -c "$CARGO_TARGET_DIR/$profile/$binary" > "/out/$binary.gz" || exit 1; \
    done
FROM scratch AS artifact
COPY --from=build /out/ /
