FROM rust:1.96-bookworm AS build
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin \
    && printf 'fn main() {}\n' > src/bin/manage.rs \
    && printf '' > src/lib.rs \
    && cargo build --release --locked --bin manage \
    && rm -rf src target/release/manage target/release/deps/httparena_reinhardt_websocket*

COPY src ./src
RUN RUSTFLAGS="-C target-cpu=native" cargo build --release --locked --bin manage

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=build /app/target/release/manage /server
COPY settings /app/settings
EXPOSE 8080
ENTRYPOINT ["/server", "runserver", "0.0.0.0:8080", "--noreload"]
