ARG TARGETARCH=amd64
FROM ghcr.io/rust-cross/rust-musl-cross:${TARGETARCH}-musl AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY assets ./assets
COPY static ./static

RUN cargo build --release && \
    musl-strip target/x86_64-unknown-linux-musl/release/statuswatch && \
    cp target/x86_64-unknown-linux-musl/release/statuswatch /statuswatch

FROM scratch
COPY --from=builder /statuswatch /statuswatch

EXPOSE 3000
VOLUME ["/data"]
ENV DATABASE_URL="sqlite:///data/statuswatch.db?mode=rwc"

ENTRYPOINT ["/statuswatch"]
