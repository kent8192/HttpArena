FROM rust:1.96-bookworm AS build
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY build.rs ./
COPY proto ./proto
RUN mkdir -p src \
    && printf 'fn main() {}\n' > src/main.rs \
    && cargo build --release --locked \
    && rm -rf src target/release/httparena-reinhardt-grpc target/release/deps/httparena_reinhardt_grpc*

COPY src ./src
RUN RUSTFLAGS="-C target-cpu=native" cargo build --release --locked

FROM debian:bookworm-slim
COPY --from=build /app/target/release/httparena-reinhardt-grpc /server
EXPOSE 8080 8443
ENTRYPOINT ["/server"]
