# Stage 1: Build the binary
FROM rust:1.75-slim AS builder
WORKDIR /app
COPY . .
RUN CARGO_BUILD_JOBS=1 cargo build --release

# Stage 2: Run the binary
FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/hmsmess_backend /app/hmsmess_backend

# Render injects the PORT environment variable dynamically
ENV PORT=10000
EXPOSE 10000

CMD ["./hmsmess_backend"]