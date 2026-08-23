FROM rust:1.96-bookworm AS build
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY build.rs ./
COPY proto ./proto
RUN mkdir -p src/bin \
    && printf 'fn main() {}\n' > src/bin/manage.rs \
    && printf '' > src/lib.rs \
    && cargo build --release --locked --bin manage \
    && rm -rf src target/release/manage target/release/deps/httparena_reinhardt_grpc*

COPY src ./src
RUN RUSTFLAGS="-C target-cpu=native" cargo build --release --locked --bin manage

FROM debian:bookworm-slim
COPY --from=build /app/target/release/manage /server
COPY settings /app/settings
EXPOSE 8080
ENTRYPOINT ["/server", "runserver", "127.0.0.1:8000", "--grpc-address", "0.0.0.0:8080", "--noreload"]
