# Reinhardt for HttpArena

This implementation is generated from a Reinhardt REST scaffold and adapted for
HttpArena's HTTP/1.1 and HTTP/2 validation profiles.

## Supported profiles

- `baseline`
- `pipelined`
- `limited-conn`
- `json`
- `json-comp`
- `json-tls`
- `upload`
- `static`
- `async-db`
- `crud`
- `fortunes`
- `api-4`
- `api-16`
- `baseline-h2`
- `static-h2`
- `baseline-h2c`
- `json-h2c`
- `gateway-64`
- `gateway-h3`

The server listens on:

- `:8080` for HTTP/1.1
- `:8081` for TLS HTTP/1.1
- `:8082` for cleartext HTTP/2 prior knowledge
- `:8443` for TLS HTTP/2

The gateway profiles use Caddy to terminate HTTP/2 or HTTP/3 on `:8443`,
serve static assets at the edge, and proxy dynamic requests to Reinhardt over
HTTP/1.1.

Native `baseline-h3` and `static-h3` are intentionally not subscribed because
Reinhardt 0.3.5 does not expose a native HTTP/3 server. HTTP/3 is covered only
by the production-style gateway profile.

## Local validation

From the repository root:

```bash
VALIDATE_TIMEOUT=900 ./scripts/validate.sh reinhardt
```
