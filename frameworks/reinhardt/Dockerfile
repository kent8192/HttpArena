FROM rust:1.96 AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin src/config \
    && printf 'fn main() {}\n' > src/bin/manage.rs \
    && printf '' > src/lib.rs \
    && cargo build --release --bin manage \
    && rm -rf src target/release/manage target/release/deps/httparena_reinhardt*
COPY src ./src
COPY settings ./settings
RUN RUSTFLAGS="-C target-cpu=native" cargo build --release --bin manage

FROM debian:trixie-slim
COPY --from=build /app/target/release/manage /server
EXPOSE 8080
CMD ["/server", "runserver", "0.0.0.0:8080", "--noreload"]
