# syntax=docker/dockerfile:1
FROM rust:1.98.1-bookworm AS build
RUN apt-get update && apt-get install -y --no-install-recommends clang libclang-dev mold cmake pkg-config python3 && rm -rf /var/lib/apt/lists/*
WORKDIR /src/game-engine-godot-conversion
ENV CARGO_TARGET_DIR=/src/game-engine-godot-conversion/target \
    CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=clang \
    RUSTFLAGS="-C link-arg=-fuse-ld=mold -C link-arg=-Wl,--thread-count=4"
COPY . /src/
ARG FIXTURE=""
RUN --mount=type=cache,id=godot-bench-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=godot-bench-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=godot-bench-target,target=/src/game-engine-godot-conversion/target,sharing=locked \
    python3 /src/refresh-source-mtimes.py /src && \
    cargo build --locked --manifest-path godot/Cargo.toml -p game-engine-godot --lib -j 16 && \
    mkdir -p /out && gzip -1 -c target/debug/libgame_engine_godot.so > /out/libgame_engine_godot.so.gz && \
    case "$FIXTURE" in \
      "") ;; \
      native_input_fixture|native_npc_visual_fixture) \
        cargo build --locked --manifest-path godot/Cargo.toml -p game-engine-network --example "$FIXTURE" -j 16 && \
        gzip -1 -c "target/debug/examples/$FIXTURE" > "/out/$FIXTURE.gz" ;; \
      *) echo "Unsupported fixture: $FIXTURE" >&2; exit 1 ;; \
    esac && du -sb target
FROM scratch AS artifact
COPY --from=build /out/ /
