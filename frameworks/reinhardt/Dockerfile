FROM rust:1.96 AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin src/config \
    && printf 'fn main() {}\n' > src/main.rs \
    && printf 'fn main() {}\n' > src/bin/manage.rs \
    && printf '' > src/lib.rs \
    && cargo build --release --bin httparena_reinhardt \
    && rm -rf src target/release/httparena_reinhardt target/release/deps/httparena_reinhardt*
COPY src ./src
COPY settings ./settings
RUN RUSTFLAGS="-C target-cpu=native" cargo build --release --bin httparena_reinhardt

FROM debian:trixie-slim
COPY --from=build /app/target/release/httparena_reinhardt /server
EXPOSE 8080 8081 8082 8443
CMD ["/server"]
