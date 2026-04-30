# authz-consumer-test

A minimal TypeScript Worker that calls `authz-worker` via Cloudflare service
binding. Used to prove the integration pattern remy-sport's Hono Worker will
follow in Phase 5.

## Status

**Source kept as a reference. The deployed Worker has been deleted** — the
proof lives in [CLAUDE.md](../../CLAUDE.md) and [CLOUDFLARE.md](../../CLOUDFLARE.md).

## What it proved

A non-Rust Worker can call `authz-worker` via `env.AUTHZ.fetch(req)` exactly
like HTTP — no network hop, identical wire format. POST/GET/DELETE round-trip
on `/debug/tuple`, plus `/health`, all worked end-to-end.

## To redeploy (if needed)

```sh
cd examples/authz-consumer-test
wrangler deploy
```

Then `curl https://authz-consumer-test.<your-subdomain>.workers.dev/probe/health`
and `/probe/tuple`.
