FROM --platform=linux/amd64 rust:1.94-slim AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY references.json ./

COPY data ./data/
RUN mkdir src
RUN echo "fn main() {}" > src/main.rs \
  # && echo "fn main() {}" > src/build_bin.rs \
  && touch src/lib.rs && cargo build --release -j 1 \
  && rm -rf src/

COPY src ./src
RUN touch src/main.rs && touch src/lib.rs && cargo build --release --bin main -j 1

FROM --platform=linux/amd64 debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/main /usr/local/bin/main
COPY configuration.yml ./

CMD ["main"]
