#
# Build stage - build binary
#
# Use latest Rust Alpine image (1.85+ required for edition2024 in some dependencies)
FROM rust:alpine AS builder
WORKDIR /app

# Install build dependencies
RUN apk add --no-cache \
        musl-dev \
        pkgconfig \
        openssl-dev \
        openssl-libs-static \
        nasm \
        build-base \
        upx

# Copy source code and build application
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

# Build with SIMD optimizations for modern x86_64 CPUs
# Default: x86-64-v2 for compatibility, can be overridden with build-arg
# For EC2 c5/c5n instances, use: --build-arg RUSTFLAGS="-C target-cpu=haswell -C target-feature=+avx2,+fma"
ARG RUSTFLAGS="-C target-cpu=x86-64-v2"
ENV RUSTFLAGS=${RUSTFLAGS}

# Optional feature selection (default uses Cargo.toml default features)
ARG MINDIA_FEATURES=default

RUN if [ "$MINDIA_FEATURES" = "minimal" ]; then \
        cargo build --release -p mindia-api --no-default-features --features minimal; \
    else \
        cargo build --release -p mindia-api; \
    fi && \
    upx --best --lzma /app/target/release/mindia-api

#
# Runtime stage
#
FROM alpine:3.19

WORKDIR /app

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates-bundle \
    ffmpeg

# Copy binary from builder
COPY --from=builder /app/target/release/mindia-api /usr/local/bin/mindia-api

# Copy migrations from build context
COPY migrations /app/migrations

EXPOSE 3000 8080

# Run the API directly
CMD ["/usr/local/bin/mindia-api"]