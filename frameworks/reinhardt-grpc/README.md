# Reinhardt gRPC for HttpArena

This entry benchmarks the gRPC surface of Reinhardt 0.4.0-alpha.8 using the
framework's `GrpcRouter` and its compatible Tonic transport.

- Plaintext HTTP/2: `:8080`
- TLS HTTP/2: `:8443`
- Service: `benchmark.BenchmarkService`

The crate follows the Reinhardt REST project layout produced by
`startproject`, with the benchmark registered as a `startapp --with-rest`
application. gRPC handlers live in `src/apps/benchmark/views.rs`; project
startup and routing stay under `src/main.rs` and `src/config/`.
The benchmark server is started through the project binary:

```bash
cargo run --bin httparena-reinhardt-grpc
```

Run validation from the repository root:

```bash
VALIDATE_TIMEOUT=900 ./scripts/validate.sh reinhardt-grpc
```
