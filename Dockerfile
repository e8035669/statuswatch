ARG TARGETARCH
# ^ no default value here: giving it one (e.g. "=amd64") sticks and silently overrides
# buildx's real per-platform value for every leg of a multi-platform build, producing
# amd64 binaries even when building linux/arm64. Confirmed via `docker buildx imagetools`/
# `file` on the extracted binary before this fix was found.

# Each arch gets its own named stage (pinned to run on the build host natively via
# rust-musl-cross's cross-toolchain). TARGETARCH only resolves to the real target arch in a
# FROM with no --platform pin of its own, so select between them with a plain FROM below
# instead of substituting ${TARGETARCH} directly into a --platform=$BUILDPLATFORM FROM
# (that resolved to the build host's arch for every leg, silently producing amd64 binaries
# even when building for linux/arm64).
FROM --platform=$BUILDPLATFORM ghcr.io/rust-cross/rust-musl-cross:${TARGETARCH}-musl AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY assets ./assets
COPY static ./static

RUN cargo build --release && \
    musl-strip target/${CARGO_BUILD_TARGET}/release/statuswatch && \
    cp target/${CARGO_BUILD_TARGET}/release/statuswatch /statuswatch

FROM scratch
COPY --from=builder /statuswatch /statuswatch

EXPOSE 3000
VOLUME ["/data"]
ENV DATABASE_URL="sqlite:///data/statuswatch.db?mode=rwc"

ENTRYPOINT ["/statuswatch"]
