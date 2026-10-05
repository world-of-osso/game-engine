# syntax=docker/dockerfile:1
FROM rust:1.98.1-bookworm AS source
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

FROM source AS build
# Only these two source trees may be refreshed, even if BUILD_PARENT contains image files.
RUN --mount=type=cache,id=server-registry-${SERVER_CACHE},target=/usr/local/cargo/registry \
    --mount=type=cache,id=server-git-${SERVER_CACHE},target=/usr/local/cargo/git \
    --mount=type=cache,id=${TARGET_CACHE},target=/build-target,sharing=locked \
    python3 -c 'import os, runpy, sys; from pathlib import Path; from types import SimpleNamespace; refresh = runpy.run_path(sys.argv[1])["refresh_sources"]; refresh(SimpleNamespace(iterdir=lambda: [Path(path) for path in sys.argv[2:]]), Path(os.environ["CARGO_TARGET_DIR"]))' \
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

# Test failures must still export the log, so the step records cargo's status instead of failing.
# The `test-data` build context (the synced mirror) binds read-only. The game-engine data sits
# beside the checkout, where the server's ground loads by default; the server data is a copy
# because SQLite writes beside world.db.
FROM source AS test
ARG TEST_ARGS
ARG TEST_RUN
RUN --mount=type=cache,id=server-registry-${SERVER_CACHE},target=/usr/local/cargo/registry \
    --mount=type=cache,id=server-git-${SERVER_CACHE},target=/usr/local/cargo/git \
    --mount=type=cache,id=${TARGET_CACHE},target=/build-target,sharing=locked \
    --mount=type=bind,from=test-data,target=/test-data \
    python3 -c 'import os, runpy, sys; from pathlib import Path; from types import SimpleNamespace; refresh = runpy.run_path(sys.argv[1])["refresh_sources"]; refresh(SimpleNamespace(iterdir=lambda: [Path(path) for path in sys.argv[2:]]), Path(os.environ["CARGO_TARGET_DIR"]))' \
        "$BUILD_PARENT/refresh-source-mtimes.py" "$BUILD_ROOT" "$BUILD_PARENT/shared-protocol" && \
    rm -rf data && cp -a /test-data/game-server data && \
    mkdir -p "$BUILD_PARENT/game-engine" && ln -sfn /test-data/game-engine "$BUILD_PARENT/game-engine/data" && \
    mkdir -p /out && echo "test run $TEST_RUN" && \
    eval "set -- $TEST_ARGS" && \
    { cargo test --locked -j "$JOBS" "$@" 2>&1; echo $? > /out/status; } | tee /out/test.log; \
    rm -rf data "$BUILD_PARENT/game-engine"
FROM scratch AS test-result
COPY --from=test /out/ /
