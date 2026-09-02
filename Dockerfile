FROM rust:latest
WORKDIR /app
COPY . .
EXPOSE 3000
CMD ["cargo", "run", "--release"]