FROM rust:latest AS builder
WORKDIR /app
COPY . .
RUN CARGO_BUILD_JOBS=1 cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/hmsmess_backend /app/hmsmess_backend
EXPOSE 3000
CMD ["./hmsmess_backend"]