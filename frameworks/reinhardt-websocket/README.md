# Reinhardt WebSocket for HttpArena

This entry benchmarks Reinhardt 0.4.0-alpha.8 WebSocket routing and message
handling through the framework's native upgrade transport.

The server listens on `ws://localhost:8080/ws` and echoes text and binary
messages while the framework handles ping/pong and close frames.

The crate follows the Reinhardt REST project layout produced by
`startproject`, with the benchmark registered as a `startapp --with-rest`
application. WebSocket handlers live in `src/apps/benchmark/views.rs`;
project startup and configuration stay under `src/main.rs` and `src/config/`.

Run validation from the repository root:

```bash
VALIDATE_TIMEOUT=900 ./scripts/validate.sh reinhardt-websocket
```
