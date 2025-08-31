FROM rust:1.89-alpine AS builder
RUN apk add --no-cache \
    musl-dev \
    pkgconfig \
    openssl-dev \
    openssl-libs-static
WORKDIR /app
COPY . .
RUN cargo build --release
RUN strip target/release/ai-web

FROM alpine:3.19
RUN apk add --no-cache libgcc openssl
COPY --from=builder /app/target/release/ai-web /ai-web
ENTRYPOINT ["/ai-web"]