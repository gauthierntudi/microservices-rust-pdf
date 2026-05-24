# syntax=docker/dockerfile:1

FROM rust:1.83-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && cargo build --release && rm -rf src
COPY src ./src
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends poppler-utils ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/authentiq-pdf-service /usr/local/bin/authentiq-pdf-service

ENV PORT=8080 \
    PDF_MAX_UPLOAD_MB=50 \
    PDF_MAX_PAGES=100 \
    PDF_DEFAULT_DPI=150 \
    PDFTOPPM_BIN=pdftoppm \
    RUST_LOG=authentiq_pdf_service=info

EXPOSE 8080
CMD ["authentiq-pdf-service"]
