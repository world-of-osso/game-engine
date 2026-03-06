FROM rust:1.92-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config \
        libclang-dev \
        libwayland-dev \
        libx11-dev \
        libx11-xcb-dev \
        libxkbcommon-x11-dev \
        libxrandr-dev \
        libxi-dev \
        libxcursor-dev \
        libasound2-dev \
        libudev-dev \
        libegl-dev \
        libgl1-mesa-dev \
        libgbm-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# This repo depends on sibling workspaces via absolute paths.
COPY . /app

RUN cargo build --release --package game-engine --bin game-engine --bin game-engine-cli


FROM ubuntu:24.04

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        xvfb \
        libwayland-client0 \
        libasound2t64 \
        libxcursor1 \
        libxkbcommon-x11-0 \
        libxi6 \
        libxrandr2 \
        libxinerama1 \
        libxfixes3 \
        libxcb1 \
        libx11-6 \
        libx11-xcb1 \
        libudev1 \
        libegl1 \
        libgl1 \
        libgbm1 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /opt/game-engine

COPY --from=builder /app/target/release/game-engine /opt/game-engine/game-engine
COPY --from=builder /app/target/release/game-engine-cli /opt/game-engine/game-engine-cli
COPY data /opt/game-engine/data

ENV GAME_SERVER_ADDR=127.0.0.1:5000

CMD ["sh", "-lc", "xvfb-run -a /opt/game-engine/game-engine --server ${GAME_SERVER_ADDR}"]
