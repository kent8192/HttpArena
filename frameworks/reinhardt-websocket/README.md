# Reinhardt WebSocket for HttpArena

This entry benchmarks Reinhardt 0.3.3 WebSocket message handling over the
framework-compatible Tokio Tungstenite transport.

The server listens on `ws://localhost:8080/ws` and echoes text and binary
messages while preserving ping payloads and close frames.

Run validation from the repository root:

```bash
VALIDATE_TIMEOUT=900 ./scripts/validate.sh reinhardt-websocket
```
