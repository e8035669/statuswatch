FROM rust:1-alpine AS builder
RUN apk add --no-cache musl-dev gcc g++ make cmake perl git pkgconfig linux-headers

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY assets ./assets
COPY static ./static

RUN cargo build --release && \
    cp target/release/statuswatch /statuswatch

FROM scratch
COPY --from=builder /statuswatch /statuswatch

EXPOSE 3000
VOLUME ["/data"]
ENV DATABASE_URL="sqlite:///data/statuswatch.db?mode=rwc"

ENTRYPOINT ["/statuswatch"]
