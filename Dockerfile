# Stage 1: Build
FROM rust:1.75-slim AS builder
WORKDIR /app
COPY . .
RUN CARGO_BUILD_JOBS=1 cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/hmsmess_backend /app/hmsmess_backend

ENV PORT=10000
EXPOSE 10000
CMD ["./hmsmess_backend"]