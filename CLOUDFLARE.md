# Cloudflare port of `authz-core`

This branch (`cloudflare`, on the [`joeblew999/authz-core`][fork] fork) deploys
the upstream [`zvectorlabs/authz-core`][upstream] engine as a Cloudflare Worker
backed by a D1-backed `TupleReader` / `TupleWriter`. The intent is to offer the
work back to upstream once it's stabilised; this doc is the at-a-glance state
for anyone reviewing it.

[fork]: https://github.com/joeblew999/authz-core/tree/cloudflare
[upstream]: https://github.com/zvectorlabs/authz-core

> **Working notes** for collaborating on this fork live in [`CLAUDE.md`](CLAUDE.md)
> (phase plan, conventions, gotchas). This file is the externally-readable
> summary; CLAUDE.md stays the working spec.

## What's deployed

| Component | Where | Notes |
|---|---|---|
| `authz-worker` | https://authz-worker.gedw99.workers.dev | The engine + `/health`, `/check`, `/debug/tuple` |
| D1 `authz-store` | binding `env.DB` on `authz-worker` | Schema in [`migrations/0001_authz_init.sql`](migrations/0001_authz_init.sql), ported from pgauthz |
| `authz-consumer-test` | https://authz-consumer-test.gedw99.workers.dev | Tiny TypeScript Worker that calls `authz-worker` via `[[services]]` binding — proves cross-language consumption works |

## What works

- ✅ **Wasm32 target.** The engine compiles for `wasm32-unknown-unknown` with one upstream change: `tokio` features `["sync", "rt"]` instead of `["sync", "rt-multi-thread"]` (the multi-thread feature is incompatible with wasm).
- ✅ **D1 storage adapter** (`crates/authz-worker/src/d1/`) implements `TupleReader` and `TupleWriter`. Every D1 call wrapped in `worker::send::SendFuture` to satisfy the `Send + Sync` trait bounds; `unsafe impl Send/Sync` on the store struct is sound because Workers isolates are single-threaded.
- ✅ **Engine over D1.** `POST /check` accepts a DSL model + check params, builds `CoreResolver::new(D1TupleStore, StaticPolicyProvider)`, calls `resolve_check`, returns `Allowed`/`Denied`/`ConditionRequired` as JSON.
- ✅ **Cross-language service binding.** Verified via the TS consumer Worker — `env.AUTHZ.fetch(req)` from a JS/TS Worker into our Rust Worker is wire-compatible HTTP, no network hop.
- ✅ **Fixture-driven validation** (`scripts/validate-fixture.sh` + `mise run prove:all`). Reads any pgauthz matrix YAML, drives setup tuples + assertions against the deployed `/check`. Currently **13 of 17 fixtures clean.**
- ✅ **Biz-model validation v1** (`scripts/biz-validate.mjs` + `mise run prove:biz`). Reads remy-sport-biz seed CSVs, generates the DSL inline, derives tuples, runs hand-crafted assertions. **6 of 7 fixtures pass** for the EVENT + PLATFORM subset. The one failure exposed an actionable design choice for the consumer (split `EVENT` into typed variants instead of conditioning by subtype) — see "Findings" below.

## What doesn't work yet

The 4 fixtures with assertion failures hit engine features that haven't been wired through this deployment:

| Fixture | Pass / Total | Cause |
|---|---|---|
| `conditions_and_context.yaml`     | 1 / 3   | CEL conditions — engine support, just not exercised here |
| `expiration_semantics.yaml`       | 1 / 3   | Temporal/expiring relations — known caveat in [CLAUDE.md](CLAUDE.md) |
| `nested_groups_exclusions.yaml`   | 25 / 29 | A few set-op edge cases in deeply nested groups |
| `setops_precedence.yaml`          | 15 / 20 | Edge cases in mixed-precedence set operations |

Plus two YAMLs (`expand_semantics`, `list_subjects_semantics`) have no `Check`-type assertions — they test other operations the runner currently skips.

## Findings (from biz validation)

| # | Symptom | Engine impact | Resolution path |
|---|---|---|---|
| 1 | `MANAGE_DIVISIONS` on `evt_003` (CAMP) returns Allowed when biz model says it should be denied (action only valid on TOURNAMENT/LEAGUE/SHOWCASE). | None — engine doesn't know about event subtypes; the biz model used CEL in CLAUDE.md's plan, but CEL coverage in the matrix is partial. | **Biz-side**: split `EVENT` into `tournament`/`league`/`showcase`/`camp` types in remy-sport-biz so the `manage_divisions` permission only exists on the first three. No CEL needed; simpler reasoning. |

Pattern is the [two-way-mirror principle](docs/tools.md): every validation failure is either an engine gap (file upstream) or a biz-model translation gap (reshape the consumer before code is written). v1 surfaced the latter.

## Runtime knobs and conventions

- **Toolchain** is managed by [mise](https://mise.jdx.dev) — see [`mise.toml`](mise.toml). Every command goes through `mise run <task>`.
- **CF resources** are prefixed `authz-` (Worker, D1, future admin app).
- **Vendored upstream sources** live in [`vendor/`](vendor/) — `pgauthz/init.sql` (schema source-of-truth), `pgauthz/matrix/*.yaml` (engine semantics fixtures). Refresh via `mise run cf:d1:upstream:fetch{,:fixtures}`. Provenance + workflow in [`vendor/README.md`](vendor/README.md).

## Observability

- `[observability] enabled = true` in `wrangler.toml`.
- Custom events emitted via `worker::console_log!` with JSON payloads.
- **Gotcha worth knowing:** Workers Observability parses each log line as JSON
  and indexes top-level keys directly — filter by `event`, `outcome`, `tuple`,
  not via `$metadata.message` (which only contains CF's auto-generated request
  line).
- Live tail: `mise run cf:tail`. Dashboard: `mise run cf:logs:dashboard`.

## Sibling Workers (composition story)

This Worker doesn't ship its own auth — it's identity-agnostic. The full deployment slots together via Cloudflare service bindings:

| Worker | Repo | Purpose |
|---|---|---|
| `auth-better-worker` | [joeblew999/auth-service](https://github.com/joeblew999/auth-service) | Identity (Better Auth v1.5 + D1 + KV). Consumer calls `env.AUTH.fetch("/auth/api/get-session")` to get a `user.id`. |
| `authz-worker` | this repo | Decisions. Consumer calls `env.AUTHZ.fetch("/check", {... subject_id: user.id})`. |
| `authz-consumer-test` | `examples/` | Demo Worker proving the binding pattern works cross-language (TS → Rust). |
| `d1-manager` | [joeblew999/d1-manager](https://github.com/joeblew999/d1-manager) | **Deployed** at https://d1-manager.gedw99.workers.dev. Account-wide D1 admin GUI behind CF Access (GitHub OAuth). |
| `kv-manager` | [joeblew999/kv-manager](https://github.com/joeblew999/kv-manager) | **Deployed** at https://kv-manager.gedw99.workers.dev. Account-wide KV admin GUI behind CF Access. Same overlay shape as `d1-manager` — `cf:provision` → `cf:access:setup` → `secrets:put-cf` → `10-deploy` → `prove:all`. Both forks share the operator's canonical fnox keys (`CLOUDFLARE_API_TOKEN`, `CF_ACCESS_GITHUB_IDP_ID`, etc.); only the per-app `*_POLICY_AUD` differs. |

Phase 5 puts `remy-sport`'s Hono Worker on top: validates session via auth, fetches the snapshot (or makes per-mutation `/check` calls) via authz, returns the answer. See CLAUDE.md "Phase 5 integration sketch" for the wiring code.

## Out of scope on this branch

- **Admin GUI** lives in a sibling repo, not here. We forked [d1-manager](https://github.com/neverinfamous/d1-manager) to [joeblew999/d1-manager](https://github.com/joeblew999/d1-manager) and deployed it as account-level ops infra at https://d1-manager.gedw99.workers.dev (behind CF Access / GitHub OAuth). It admins every D1 on the operator's account — `authz-store` and any other DB. Visit it directly; nothing in this repo deploys or maintains it.
- **Loading authorization policies from D1.** `/check` accepts the DSL inline. Storing/retrieving from `authz_authorization_policy` is Phase 4 work.
- **`/snapshot/:user_type/:user_id`** — the bulk permission map for downstream consumer caching. Phase 5.
- **AuthZEN endpoints.** Not addressed here.

## How upstream might receive this

The fork is structured to minimise diff against upstream:

| Path | Status |
|---|---|
| `src/**` (engine) | Untouched. |
| `Cargo.toml` | Two surgical changes: workspace block at top, `tokio` features (the latter is the only behavior-affecting wasm-compat fix). |
| `Makefile`, `.githooks/`, `rust-toolchain.toml` | Untouched. |
| All new code | Lives in `crates/authz-worker/` (`publish = false`, excluded from default-members so `cargo publish` on the engine stays clean). |
| All new tooling | `mise.toml`, `vendor/`, `scripts/`, `migrations/`, `examples/`, `.github/`, `docs/` — additive only. |

If upstream wants any of this, the worker crate + migrations + vendor pattern can drop in as a sibling crate without touching the engine. If they don't want it, this fork stays useful to the [remy-sport][remy-sport] / [remy-sport-biz][remy-sport-biz] consumer projects that drove this work.

[remy-sport]: https://github.com/joeblew999/remy-sport
[remy-sport-biz]: https://github.com/joeblew999/remy-sport-biz
