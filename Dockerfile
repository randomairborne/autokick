FROM rust:alpine AS builder

WORKDIR /build
COPY . .

RUN apk add musl-dev

RUN \
    --mount=type=cache,target=/build/target/ \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    cargo build --release && cp /build/target/release/autokick /build/autokick

FROM alpine:latest

WORKDIR /

COPY --from=builder /build/autokick /usr/bin/autokick

ENTRYPOINT "/usr/bin/autokick"

