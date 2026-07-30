FROM rust:1.96-bookworm AS build
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
    && printf 'fn main() {}\n' > src/main.rs \
    && cargo build --release --locked \
    && rm -rf src target/release/httparena-reinhardt-websocket target/release/deps/httparena_reinhardt_websocket*

COPY src ./src
RUN RUSTFLAGS="-C target-cpu=native" cargo build --release --locked

FROM debian:bookworm-slim
COPY --from=build /app/target/release/httparena-reinhardt-websocket /server
EXPOSE 8080
ENTRYPOINT ["/server"]
