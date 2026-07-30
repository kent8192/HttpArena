# Reinhardt gRPC for HttpArena

This entry benchmarks the gRPC surface of Reinhardt 0.3.3 using the
framework's gRPC configuration and its compatible Tonic transport.

- Plaintext HTTP/2: `:8080`
- TLS HTTP/2: `:8443`
- Service: `benchmark.BenchmarkService`

The crate follows the Reinhardt REST project layout produced by
`startproject`, with the benchmark registered as a `startapp --with-rest`
application. gRPC handlers live in `src/apps/benchmark/views.rs`; project
startup and configuration stay under `src/main.rs` and `src/config/`.

Run validation from the repository root:

```bash
VALIDATE_TIMEOUT=900 ./scripts/validate.sh reinhardt-grpc
```
