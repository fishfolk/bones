# syntax=docker/dockerfile:1.4

#
# Bones Matchmaker Docker image
#

FROM rust:1.64-slim as builder

RUN apt-get update && \
    apt-get install -y \
        curl \
        pkg-config \
        libudev-dev \
        libasound2-dev && \
        rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/bones
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry/cache \
    --mount=type=cache,target=/usr/local/cargo/registry/index \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/src/bones/target \
    cargo build -p bones_matchmaker

RUN --mount=type=cache,target=/usr/src/bones/target \
    cp target/debug/bones_matchmaker /usr/local/bin/bones_matchmaker

# TODO: Slim down this container. We need to try and strip all unneeded deps from Bevy for the
# matchmaker.
FROM debian:bullseye
RUN apt-get update && apt-get install -y libasound2 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/bin/bones_matchmaker /usr/local/bin/bones_matchmaker
EXPOSE 8943/udp
ENTRYPOINT /usr/local/bin/bones_matchmaker
