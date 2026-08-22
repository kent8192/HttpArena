FROM rust:1.96-bookworm AS build
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
    && printf 'fn main() {}\n' > src/main.rs \
    && cargo build --release --locked --bin httparena-reinhardt-websocket \
    && rm -rf src target/release/httparena-reinhardt-websocket target/release/deps/httparena_reinhardt_websocket*

COPY src ./src
RUN RUSTFLAGS="-C target-cpu=native" cargo build --release --locked --bin httparena-reinhardt-websocket

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=build /app/target/release/httparena-reinhardt-websocket /server
COPY settings /app/settings
EXPOSE 8080
ENTRYPOINT ["/server", "runserver", "0.0.0.0:8080", "--noreload"]
