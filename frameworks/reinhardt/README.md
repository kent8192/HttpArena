# Reinhardt for HttpArena

This implementation is generated from a Reinhardt REST scaffold and adapted for
HttpArena's HTTP/1.1 validation profiles.

## Supported profiles

- `baseline`
- `pipelined`
- `limited-conn`
- `json`
- `json-comp`
- `upload`
- `static`
- `async-db`
- `crud`
- `fortunes`
- `api-4`
- `api-16`
- `gateway-64`
- `gateway-h3`

The `manage runserver` entry point serves the project's `UnifiedRouter` on
`:8080` for HTTP/1.1.

The gateway profiles use Caddy to terminate HTTP/2 or HTTP/3 on `:8443`,
serve static assets at the edge, and proxy dynamic requests to Reinhardt over
HTTP/1.1.

Native HTTP/2, TLS, and HTTP/3 profiles are intentionally not subscribed because
`manage runserver` does not expose those listeners. TLS and HTTP/3 are covered by
the production-style gateway profiles.

## Local validation

From the repository root:

```bash
VALIDATE_TIMEOUT=900 ./scripts/validate.sh reinhardt
```
