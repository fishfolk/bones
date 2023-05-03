# syntax=docker/dockerfile:1.4

#
# Bones Matchmaker Docker image
#

FROM rust:1.69.0-slim as builder

WORKDIR /usr/src/bones
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry/cache \
    --mount=type=cache,target=/usr/local/cargo/registry/index \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/src/bones/target \
    # Uncomment if building with behind custom CA cert
    #--mount=type=bind,src=./cacert.gitignore.crt,target=/etc/ssl/certs/ca-certificates.crt \
    cargo build --release -p bones_matchmaker

RUN --mount=type=cache,target=/usr/src/bones/target \
    cp target/release/bones_matchmaker /usr/local/bin/bones_matchmaker

FROM ubuntu:23.04
USER 1001
COPY --from=builder /usr/local/bin/bones_matchmaker /bones_matchmaker
EXPOSE 8943/udp
CMD /bones_matchmaker
