# Reinhardt gRPC for HttpArena

This entry benchmarks the gRPC surface of Reinhardt 0.4.0-alpha.8 using the
framework's `GrpcRouter` and its compatible Tonic transport.

- Plaintext HTTP/2: `:8080`
- Service: `benchmark.BenchmarkService`

The crate follows the Reinhardt REST project layout produced by
`startproject`, with the benchmark registered as a `startapp --with-rest`
application. gRPC handlers live in `src/apps/benchmark/views.rs`; project
startup and routing stay under `src/bin/manage.rs` and `src/config/`.
The benchmark server is started through Reinhardt's management command:

```bash
cargo run --bin manage -- runserver 127.0.0.1:8000 --grpc-address 0.0.0.0:8080
```

Run validation from the repository root:

```bash
VALIDATE_TIMEOUT=900 ./scripts/validate.sh reinhardt-grpc
```
