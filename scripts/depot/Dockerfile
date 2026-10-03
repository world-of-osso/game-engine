# syntax=docker/dockerfile:1
FROM rust:1.98.1-bookworm AS source
RUN apt-get update && apt-get install -y --no-install-recommends clang libclang-dev mold cmake pkg-config python3 && rm -rf /var/lib/apt/lists/*
WORKDIR /src/game-engine-godot-conversion
ARG TARGET_CACHE
ENV CARGO_TARGET_DIR=/src/game-engine-godot-conversion/target \
    CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=clang \
    RUSTFLAGS="-C link-arg=-fuse-ld=mold -C link-arg=-Wl,--thread-count=4"
COPY --exclude=test-assets . /src/

FROM source AS build
ARG FIXTURE=""
ARG CLI=""
RUN --mount=type=cache,id=godot-bench-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=godot-bench-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=${TARGET_CACHE},target=/src/game-engine-godot-conversion/target,sharing=locked \
    python3 /src/refresh-source-mtimes.py /src && \
    cargo build --locked --manifest-path godot/Cargo.toml -p game-engine-godot --lib -j 16 && \
    mkdir -p /out && gzip -1 -c target/debug/libgame_engine_godot.so > /out/libgame_engine_godot.so.gz && \
    case "$FIXTURE" in \
      "") ;; \
      *) \
        cargo build --locked --manifest-path godot/Cargo.toml -p game-engine-network --example "$FIXTURE" -j 16 && \
        gzip -1 -c "target/debug/examples/$FIXTURE" > "/out/$FIXTURE.gz" ;; \
    esac && \
    if [ -n "$CLI" ]; then \
        cargo build --locked --manifest-path godot/Cargo.toml -p game-engine-cli --bin game-engine-cli -j 16 && \
        gzip -1 -c target/debug/game-engine-cli > /out/game-engine-cli.gz; \
    fi && du -sb target
FROM scratch AS artifact
COPY --from=build /out/ /

# Test failures must still export the log, so the step records cargo's status instead of failing.
# The assets bind read-only and tests write to a local copy: a writable bind of the context
# made BuildKit re-send all of it on the next build instead of only changed files.
FROM source AS test
ARG TEST_ARGS
ARG TEST_RUN
RUN --mount=type=cache,id=godot-bench-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=godot-bench-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=${TARGET_CACHE},target=/src/game-engine-godot-conversion/target,sharing=locked \
    --mount=type=bind,source=test-assets,target=/test-assets \
    python3 /src/refresh-source-mtimes.py /src && \
    rm -rf data && cp -a /test-assets data && \
    ln -sfn ../../data godot/core/data && mkdir -p /out && echo "test run $TEST_RUN" && \
    eval "set -- $TEST_ARGS" && \
    { cargo test --locked --manifest-path godot/Cargo.toml -j 16 "$@" 2>&1; echo $? > /out/status; } | tee /out/test.log; \
    rm -rf data
FROM scratch AS test-result
COPY --from=test /out/ /
