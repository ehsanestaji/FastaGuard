FROM rust:1-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY schema ./schema
COPY src ./src
RUN cargo build --release --locked

FROM debian:bookworm-slim

LABEL org.opencontainers.image.title="FastaGuard"
LABEL org.opencontainers.image.description="FASTA preflight QC for assembly pipelines"
LABEL org.opencontainers.image.licenses="MIT"

RUN useradd --create-home --shell /usr/sbin/nologin fastaguard
COPY --from=builder /app/target/release/fastaguard /usr/local/bin/fastaguard

USER fastaguard
WORKDIR /data
ENTRYPOINT ["fastaguard"]
