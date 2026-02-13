#
# Build stage - build binary
#
# Default to static MUSL target for maximum compatibility
ARG RUST_TARGET="x86_64-unknown-linux-musl"

# Use latest Rust Alpine image (1.85+ required for edition2024 in some dependencies)
FROM rust:alpine AS builder
ARG RUST_TARGET
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

# Build target (MUSL for static linking)
RUN rustup target add ${RUST_TARGET}

# Copy source code and build application
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY migrations ./migrations

# Build with SIMD optimizations for modern x86_64 CPUs
# Default: x86-64-v2 for compatibility, can be overridden with build-arg
# For EC2 c5/c5n instances, use: --build-arg RUSTFLAGS="-C target-cpu=haswell -C target-feature=+avx2,+fma"
ARG RUSTFLAGS="-C target-cpu=x86-64-v2"
ENV RUSTFLAGS=${RUSTFLAGS}

# Optional feature selection (default uses Cargo.toml default features)
ARG MINDIA_FEATURES=default

RUN if [ "$MINDIA_FEATURES" = "minimal" ]; then \
        cargo build --release -p mindia-api --no-default-features --features minimal --target ${RUST_TARGET}; \
    else \
        cargo build --release -p mindia-api --target ${RUST_TARGET}; \
    fi && \
    upx --best --lzma /app/target/${RUST_TARGET}/release/mindia-api

#
# Runtime stage
#
FROM alpine:3.19
ARG RUST_TARGET

WORKDIR /app

# Use a more reliable APK mirror to avoid intermittent dl-cdn issues
RUN sed -i 's/dl-cdn.alpinelinux.org/mirrors.edge.kernel.org/g' /etc/apk/repositories

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates-bundle \
    ffmpeg

# Create an unprivileged user to run the service
RUN addgroup -S mindia && adduser -S mindia -G mindia

# Copy binary from builder
COPY --from=builder /app/target/${RUST_TARGET}/release/mindia-api /usr/local/bin/mindia-api

# Copy migrations from build context
COPY migrations /app/migrations

# Ensure runtime files are owned by the unprivileged user
RUN chown -R mindia:mindia /app

# Drop privileges
USER mindia:mindia

EXPOSE 3000 8080

# Run the API directly
CMD ["/usr/local/bin/mindia-api"]