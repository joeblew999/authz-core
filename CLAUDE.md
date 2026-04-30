# Notes for Claude

This is **joeblew999's fork** of [zvectorlabs/authz-core](https://github.com/zvectorlabs/authz-core).
Work happens on the **`cloudflare`** branch — never push to `main` (we want to track upstream cleanly).

## Why this fork exists

This fork exists to power the authorisation layer for the **remy-sport / remy-sport-biz** project (a basketball tournament platform). The biz model in [`../remy-sport-biz/`](../remy-sport-biz/) defines:
- 6 object types (EVENT, TEAM, PLAYER, ORG, DIVISION, PLATFORM)
- ~14 relations (OWNER, HEAD_COACH, GUARDIAN, FOLLOWER_*, PLATFORM_ADMIN, PUBLIC, …)
- ~70 actions (CREATE_EVENT, EDIT_EVENT, MANAGE_ROSTER, …)
- Permission tuples scoped sometimes by event subtype (TOURNAMENT/LEAGUE/SHOWCASE)

See [`../remy-sport-biz/decisions/decision-002-authorisation-engine.md`](../remy-sport-biz/decisions/decision-002-authorisation-engine.md) for the full ADR. We chose authz-core because the biz model is fully expressible in its DSL (modulo two known caveats: temporal relations and event-subtype scoping via CEL).

The engine and this Worker are **generic** — they don't bake in remy-sport concepts. The remy-sport project will load its own DSL and tuples at runtime.

## Consumers (downstream architecture)

Two existing apps in `../remy-sport/` will consume this Worker:

```
┌──────────────────────────┐      ┌──────────────────────────┐      ┌────────────────────────────┐
│ remy-sport React SPA     │      │ remy-sport Hono Worker   │      │ authz-worker (THIS REPO)   │
│ Cloudflare Pages         │ ───► │ Cloudflare Workers       │ ───► │ Cloudflare Workers         │
│ remy-sport-design.pages  │ HTTP │ remy-sport.workers.dev   │ HTTP │ authz-worker.workers.dev   │
│ /me/permissions          │      │ owns business logic +    │      │ owns: tuples, DSL, checks  │
│ uses snapshot for UI     │      │ session, calls authz     │      │ no business logic          │
└──────────────────────────┘      └──────────────────────────┘      └────────────────────────────┘
```

**Pattern: server-side snapshot, client-side hydration.**
- React **never** calls authz-worker directly. It calls remy-sport's Hono Worker.
- Hono fetches a permission **snapshot** from authz-worker (one big flat JSON of every grant for the user) and embeds it in the `/me` response.
- React holds the snapshot in a context provider; UI gating is **synchronous**, **zero-latency**, **stale-until-refetch**.
- Mutations still hit Hono → authz-worker `/check` per action — snapshot is for UX, not security.

This means **authz-worker exposes two main HTTP endpoints** for consumers:
- `POST /check` — single check, returns Allowed/Denied/ConditionRequired (used per-mutation).
- `GET /snapshot/:user_type/:user_id` — flat permission map for one user (used at login/session-refresh; React-shaped).

Both endpoints are the responsibility of **Phase 5** (consumer integration). Phases 1–4 build the engine and validate it; Phase 5 wires it to remy-sport's existing apps.

## Upstream relationship

We don't yet know whether the upstream author ([zvectorlabs](https://github.com/zvectorlabs)) wants any of this work. The plan is:

1. Get Phases 1–3 working on the `cloudflare` branch of this fork.
2. Once it's running cleanly, reach out to upstream and offer the work back.
3. Until then, this is **a private fork** — don't open PRs upstream, don't assume they'll accept anything.

This shapes the rules below: keep the fork minimal, surgical, and easy to read. Every change should be defensible as either (a) a wasm-compat fix the engine genuinely needs, or (b) a **separate** Worker/D1/admin crate that doesn't touch upstream code at all. If we have to refactor engine internals, flag it and minimise the diff.

If upstream declines the contribution, we keep the fork and rebase. Either outcome is fine — just don't burn bridges by making the fork hard to compare.

## Open this repo? Start here.

Fresh session, just opened the repo? Do this in order:

1. `git branch --show-current` → must be `cloudflare`. If not: `git checkout cloudflare`.
2. Read this file end-to-end (you're doing it).
3. Read [`mise.toml`](mise.toml) — every command goes through `mise run`.
4. Read [`Cargo.toml`](Cargo.toml) — the actual deps, not what you remember.
5. `mise install` — installs everything: rust, node, jq, **wrangler** (via `npm:` backend), **worker-build** (via `cargo:` backend).
6. `mise run setup` — adds the `wasm32-unknown-unknown` rust target.
7. `mise run cf:whoami` — confirm logged into Cloudflare account `gedw99`. If not: `mise run cf:login`.
8. Now follow [Process](#process) below.

## Phasing — five sessions, in order

| Phase | Status | Goal | When complete |
|---|---|---|---|
| **1** | ✅ done | Worker compiles for wasm32, deploys to CF, `/health` returns 200. Engine wired to **in-memory/stub store**. | `mise run worker:health:remote` → `{"ok": true}` |
| **2** | ✅ done | Replace stub with **D1-backed `TupleReader`/`TupleWriter`**, schema ported from pgauthz, smoke-test round-trips a tuple. | `/debug/tuple` POST→GET→DELETE round-trips against deployed D1. |
| **3 Part A** | ✅ done | `POST /check` runs the engine over D1 tuples — load-bearing for downstream consumers. | `mise run prove:simple-direct` → 4/4 PASS against deployed Worker. |
| **3 Part B** | ❌ moved out | d1-manager is account-level ops infra (admins every D1 on the account, account-scoped API token), not authz-core's concern. Deploy from a separate ops repo or one-off. See Phase 3 Part B section for the conclusion. | n/a — no longer in this repo's scope. |
| **4 v1** | ✅ done | **Biz-model validation v1** — load remy-sport-biz CSVs as DSL (EVENT + PLATFORM types), seed tuples, run hand-crafted fixtures. | `mise run prove:biz` → 6/7 PASS, single failure is the predicted CAMP/MANAGE_DIVISIONS gap. |
| **4 v2** | ⌛ next | Extend biz validation to **TEAM, PLAYER, ORG**, GUARDIAN logic, temporal `TEAM_PLAYER` expiry, full Phase-4 fixture table. | All cases from CLAUDE.md's Phase 4 fixture table covered (15+ assertions). |
| **5** | ⌛ later | **Consumer integration** — `/check` and `/snapshot/:user_type/:user_id` HTTP endpoints, hooked from remy-sport's Hono Worker, with the React SPA gating UI off the snapshot. Also wires in [auth-service](https://github.com/joeblew999/auth-service) (Better Auth on a Worker — `auth-better-worker`) for identity. | React app at `remy-sport-design.pages.dev` correctly hides/shows buttons based on logged-in user's snapshot from `authz-worker`, with auth from `auth-better-worker`. |

Each phase below has its own self-contained spec. Don't skip ahead — Phase 3 (admin) assumes Phase 2 (D1); Phase 4 (biz validation) assumes Phase 2; Phase 5 (consumer integration) assumes Phase 4.

> External-readable summary lives in [CLOUDFLARE.md](CLOUDFLARE.md). Keep both files honest.

### Current state (live)

- **Worker:** `authz-worker` deployed at https://authz-worker.gedw99.workers.dev — `/health`, `/check`, `/debug/tuple`. workers-rs `worker = "0.8.1"`, `compatibility_date = "2026-04-01"`, `[observability] enabled = true`.
- **D1:** `authz-store` (id `822c5996-98dd-4d10-a77d-805f870d92e6`), schema in [migrations/0001_authz_init.sql](migrations/0001_authz_init.sql), bound as `env.DB`.
- **Engine coverage** (`mise run prove:all` against the deployed `/check`): **13 of 17 vendored matrix fixtures clean.** Failures concentrated in CEL conditions, temporal relations, and a few set-op edge cases — all known/expected.
- **Biz validation** (`mise run prove:biz`): **6/7 PASS for v1 (EVENT + PLATFORM).** The single failure is `owner_cannot_manage_divisions_on_camp` — exactly the predicted event-subtype-scoping caveat. **Resolution is biz-side, not engine-side**: split `EVENT` into `tournament`/`league`/`showcase`/`camp` types so `manage_divisions` only exists on the first three. No CEL needed.
- **Service binding proven**: `examples/authz-consumer-test/` (TS) calls our Rust Worker via `env.AUTHZ.fetch(req)`. Same pattern remy-sport's Hono Worker uses in Phase 5.
- **Vendored upstream:** [vendor/pgauthz/init.sql](vendor/pgauthz/init.sql) (schema source-of-truth), [vendor/pgauthz/matrix/](vendor/pgauthz/matrix/) (17 fixture YAMLs). Provenance + refresh in [vendor/README.md](vendor/README.md).
- **Observability gotcha**: filter `console.log` JSON top-level keys directly (e.g. `event`, `outcome`, `tuple`) — *not* via `$metadata.message`, which only contains CF's auto-generated request line.
- **CI:** [.github/workflows/cloudflare-ci.yml](.github/workflows/cloudflare-ci.yml) — `workflow_dispatch` only (off-but-ready). [.github/dependabot.yml](.github/dependabot.yml) — scans cargo + actions, `open-pull-requests-limit: 0` (off-but-ready).

### Phase 5 integration sketch — auth + authz composition

Phase 5 wires three Workers together via service bindings (no public network hops between them — all in-isolate fetch dispatch).

```
                  ┌────────────────────────────┐
                  │ remy-sport Hono Worker     │ ← consumer / orchestrator
                  └──────┬──────────────┬──────┘
        env.AUTH.fetch() │              │ env.AUTHZ.fetch()
                         ▼              ▼
   ┌─────────────────────────┐  ┌──────────────────────────┐
   │ auth-better-worker      │  │ authz-worker (this repo) │
   │ Better Auth v1.5        │  │ engine + D1 tuples       │
   │ joeblew999/auth-service │  │                          │
   │                         │  │                          │
   │ /auth/api/get-session   │  │ POST /check              │
   │ /auth/api/organization/ │  │ GET  /snapshot/:t/:id    │
   │   get-active-member     │  │                          │
   └─────────────────────────┘  └──────────────────────────┘
```

Pattern (TypeScript inside Hono Worker):

```ts
// 1. Identity from auth-service
const sess = await env.AUTH.fetch("https://auth/auth/api/get-session", { headers: req.headers });
const user = (await sess.json())?.user;
if (!user) return new Response("unauthorized", { status: 401 });

// 2. Decision from authz-worker
const dec = await env.AUTHZ.fetch("https://authz/check", {
  method: "POST",
  headers: { "content-type": "application/json" },
  body: JSON.stringify({
    model, object_type, object_id, relation,
    subject_type: "user", subject_id: user.id,
  }),
});
```

Org/role context: `get-active-member` returns role-based org membership. The Hono Worker can map this onto PLATFORM-type tuples (`ANY_ORGANIZER`, `PLATFORM_ADMIN`) without persisting auth identity in `authz-store` — derived on the fly per request.

Cross-repo concern split:
- **auth-service** (Better Auth Worker) owns: sessions, OAuth, magic links, password reset, email verification.
- **authz-worker** (this repo) owns: tuples, DSL, decisions. Identity-agnostic — takes any string `subject_id`.
- **Hono Worker** owns: composition. Both service bindings, request orchestration, snapshot caching.

### Perf path / when to scale

The architecture is naturally well-aligned with low-latency authz reads — most of the work is read-side and the inputs are stable.

| Lever | What it buys | When to flip |
|---|---|---|
| **Model is stable.** Type system + DSL is parsed/compiled once per request today; can be cached `Arc<TypeSystem>` once we want to. | Resolver itself is fast — bottleneck is D1 reads, not engine work. | Already the default; revisit only if profiling shows model-parse cost. |
| **`/snapshot` cached at the consumer.** Hono Worker fetches once at login, embeds in `/me`, React holds in context. | Per-render UI gating is zero-RTT — `if (snapshot.can_edit_event) {…}`. | Phase 5 — design choice already made. |
| **D1 read replication.** Off today (`wrangler d1 info` shows `read_replication.mode: disabled`). One-flag flip. | Reads served from nearest region (~5ms instead of ~50ms cross-continent). Writes still go to primary. | When measurable traffic appears from a second continent. Pre-launch is too early. |
| **D1 Sessions API.** Bookmark-based read-after-write consistency. | Use case: tuple write → immediate `/check` on the same request. | Only if integration shows we're hitting the rare "wrote, then read stale" race. Not on remy's expected hot path. |
| **`/check` result cache** in `authz-worker`. CoreResolver supports an L2 cache (today wired to `noop_cache`). | Repeated identical checks within a TTL get an in-memory hit, skip D1 entirely. | After Phase 5 is live and we see the actual repeat-check pattern. |
| **Snapshot pre-compute** (eventual). Re-derive on tuple write, store in `authz_snapshot` table, serve raw. | Login becomes a single-row D1 read. | Only if cold-snapshot latency becomes a UX issue. |

**What remains genuinely unknown** (not solved by any of the above) is the **shape** of the snapshot for power-user accounts (federation-wide PLATFORM_ADMIN with thousands of objects). Likely answer is "wildcard the snapshot for that role", but worth measuring at integration time.

## Goal — Phase 1 (this session)

Get this fork compiling for `wasm32-unknown-unknown` and deployed as a Cloudflare Worker. That's it. No D1, no GUI, no AuthZEN endpoints. Just `GET /health` returning `{"ok": true}` from a Worker linked against the engine, with an in-memory or stub `TupleReader`/`TupleWriter`.

## Conventions (inherited from joeblew999 projects)

- All tasks via `mise run …`. Never call `cargo`/`wrangler`/`worker-build` directly — add a task instead.
- Mise tasks idempotent where possible (use `sources = […]`).
- All Cloudflare resources prefixed `authz-…` (`authz-worker` is the CF Worker name).
- Subdomain is `gedw99.workers.dev`.
- Rust toolchain pinned in two places that must agree: `rust-toolchain.toml` (channel = `1.93`) and `mise.toml` (`rust = "1.93"`). If you bump one, bump the other.

## Files to leave alone (upstream's, not ours)

- `src/**` — the engine. Touch only for wasm-compat fixes, and flag them.
- `Makefile` — upstream's dev Makefile. Don't extend it; add tasks to `mise.toml` instead.
- `.githooks/` — upstream's commit hooks. Leave alone.
- `rust-toolchain.toml` — fine to keep agreement with `mise.toml`, otherwise leave alone.
- `Cargo.toml` `[package]` and `[dependencies]` — only edit for wasm-compat (e.g. tokio features). Don't add deps unless required for wasm.

## Mise task cheat sheet

`wrangler` and `worker-build` are managed by mise itself (`npm:` and `cargo:` backends in `[tools]`). No bun, no global cargo install — `mise install` brings everything.

```
mise install              # installs rust, node, jq, wrangler, worker-build
mise run setup            # adds wasm32 rust target
mise run check            # cargo check (native)
mise run check:wasm       # cargo check --target wasm32-unknown-unknown
mise run worker:check     # check the worker crate against wasm32
mise run worker:build     # worker-build --release inside crates/authz-worker
mise run worker:dev       # wrangler dev — local at :8787
mise run worker:health    # curl localhost:8787/health
mise run cf:deploy        # wrangler deploy
mise run worker:health:remote  # curl deployed /health
mise run cf:tail          # tail live worker logs
mise run cf:whoami        # confirm CF auth
mise run cf:login         # authenticate wrangler with CF account
```

## Layout (planned)

```
authz-core/                     ← root crate (the engine, kept publishable)
├── src/                        ← engine code, untouched
├── crates/
│   └── authz-worker/           ← NEW: workers-rs crate, depends on engine via path
│       ├── Cargo.toml
│       ├── src/lib.rs
│       └── wrangler.toml
├── Cargo.toml                  ← becomes a workspace
├── mise.toml
└── CLAUDE.md
```

The worker crate is `publish = false` and excluded from default-members so `cargo publish` on the root crate stays clean.

## Known wasm blockers (hypotheses — verify before fixing)

These came from a prior chat with another Claude session. **Treat as hypotheses, not facts** — run `mise run check:wasm` first and surface the actual errors before applying any of these.

1. **`tokio` features** — root `Cargo.toml` line 19 has `features = ["sync", "rt-multi-thread"]`. `rt-multi-thread` is not supported on `wasm32-unknown-unknown`. Drop it. Tokio's wasm-supported features are `sync, macros, io-util, rt, time`. *(Highest probability blocker.)*
2. **`cel = "0.12.0"`** — unverified for wasm32. May pull in deps that need wasm features. Inspect compiler errors before assuming.
3. **`getrandom` (transitive)** — typically needs the `js` feature for wasm32. May surface via `serde_json` or `cel`.
4. **`chrono`/`time`** — if they appear transitively, may need wasm features.
5. **Workers types and `Send + Sync`** — D1/KV futures are not `Send`. Wrap in `worker::send::SendFuture` if needed. Not relevant until we add D1.

**Rule:** Don't pre-fix anything except #1. For everything else, hit the error, then fix.

## Process

Strict stop-at-failure. **Show diffs and outputs as you go, not at the end.** If anything fails, stop, show the exact error, ask before working around it.

1. `mise run setup` — installs wasm target + worker-build.
2. `mise run check` — native check (sanity baseline; should pass on bare fork).
3. `mise run check:wasm` — show output. **Stop here on failure** before fixing anything. Then apply fixes one at a time (tokio first; see [Known wasm blockers](#known-wasm-blockers-hypotheses--verify-before-fixing)). Re-run after each fix.
4. Convert root `Cargo.toml` to a workspace. The minimum viable diff:
   ```toml
   [workspace]
   members  = [".", "crates/*"]
   resolver = "2"

   # existing [package] and [dependencies] stay as-is — the root crate is still authz-core
   ```
   This makes the root crate a workspace member alongside future `crates/*`. Verify `cargo check` still passes.
5. Create `crates/authz-worker/` (workers-rs + `#[event(fetch)]` + one route `GET /health`). `Cargo.toml` must have `publish = false` and depend on the engine via `authz-core = { path = "../.." }`. Stub in-memory `TupleReader`/`TupleWriter` (~30 LOC, `HashMap<String, Vec<Tuple>>`). Don't reference D1 yet.
6. Create `crates/authz-worker/wrangler.toml`:
   ```toml
   name                = "authz-worker"
   main                = "build/worker/shim.mjs"
   compatibility_date  = "2025-09-01"
   workers_dev         = true
   [build]
   command = "worker-build --release"
   ```
7. `mise run worker:check` (compile worker against wasm32) → `mise run worker:dev` → `mise run worker:health`. Local should return `{"ok": true}`.
8. `mise run cf:deploy` → `mise run worker:health:remote`. Show the deployed URL and curl output.
9. Stop.

## Phase 2: D1 storage adapter (after Phase 1 is deployed)

Once `/health` is live with the stub store, replace `TupleReader`/`TupleWriter` with a real D1-backed adapter. Same `cloudflare` branch, same fork.

### Goal — Phase 2

A working D1 storage adapter for authz-core: schema migrated, adapter wired into the Worker, and a smoke-test endpoint that round-trips a tuple write+read against a real D1 database (local first, then deployed).

### Stack — fixed, do not deviate

- D1 via `worker::d1::D1Database`, accessed through the `Env` binding.
- **Every D1 call wrapped in `worker::send::SendFuture`** because `JsFuture` is `!Send` and authz-core's `TupleReader`/`TupleWriter` traits require `Send + Sync`.
- `unsafe impl Send for D1TupleStore {}` and `unsafe impl Sync for D1TupleStore {}` on the adapter struct. Sound because Workers isolates are single-threaded — the unsafe impls just satisfy the trait bound that authz-core declares for portability with multi-threaded servers.
- Schema ported from pgauthz: <https://github.com/zvectorlabs/pgauthz/blob/main/crates/pgauthz/sql/init.sql>
- Migrations live in `migrations/` and are applied via `wrangler d1 migrations apply`.

### Schema port rules (pgauthz Postgres → D1 SQLite)

Apply these transforms mechanically. Don't get clever.

| pgauthz (Postgres) | D1 (SQLite) |
|---|---|
| `CREATE SCHEMA authz; CREATE TABLE authz.tuple ...` | drop `authz.` prefix, rename to `authz_tuple` |
| `TIMESTAMPTZ` | `TEXT` (ISO-8601 strings) — pick TEXT for debuggability |
| `JSONB` | `TEXT` |
| `gen_random_uuid()` | application-side UUID generation in Rust (`uuid` crate, `wasm-bindgen` feature) |
| `CREATE INDEX ... INCLUDE (...)` | drop the `INCLUDE` clause; SQLite has no covering indexes |
| `ANALYZE authz.tuple;` | `ANALYZE authz_tuple;` (works as-is) |
| Postgres-specific functions (`now()`, `uuid_generate_v4()`) | replace with parameter binding from Rust |

### Layout to add

```
migrations/
  0001_authz_init.sql           ported pgauthz schema
crates/authz-worker/src/d1/
  mod.rs                        pub use D1TupleStore
  store.rs                      struct D1TupleStore { db: D1Database }
  reader.rs                     impl TupleReader for D1TupleStore
  writer.rs                     impl TupleWriter for D1TupleStore
  sql.rs                        const SELECT_TUPLE: &str = "..."; etc.
crates/authz-worker/tests/
  d1_roundtrip.rs               smoke: write tuple → read it back
```

In `crates/authz-worker/wrangler.toml`:

```toml
[[d1_databases]]
binding       = "DB"
database_name = "plat-authz"
database_id   = "<set by wrangler d1 create>"
```

In the Worker entry point: construct the store from `env.d1("DB")?` and pass it where the in-memory/stub store was wired in Phase 1.

### Adapter implementation pattern (the part that bites if you skip it)

`TupleReader` and `TupleWriter` are declared `Send + Sync`. `worker::d1::D1Database` returns `!Send` futures (the `JsFuture` machinery holds `Rc<RefCell<...>>`). The obvious adapter won't compile.

The fix is the workers-rs-blessed `SendFuture` wrapper. Pattern for every async D1 call:

```rust
use worker::send::SendFuture;

#[async_trait::async_trait]
impl TupleReader for D1TupleStore {
    async fn read(&self, key: &TupleKey) -> Result<Option<Tuple>, AuthzError> {
        SendFuture::new(async move {
            let row = self.db
                .prepare(sql::SELECT_TUPLE)
                .bind(&[key.subject.into(), key.relation.into(), key.resource.into()])?
                .first::<TupleRow>(None)
                .await?;
            Ok(row.map(Into::into))
        }).await
    }
}

unsafe impl Send for D1TupleStore {}
unsafe impl Sync for D1TupleStore {}
```

The `unsafe impl` lines are **not optional**. Without them the adapter doesn't satisfy authz-core's trait bounds and the Worker won't link. If you fight the type checker beyond what `SendFuture` solves, **stop and show the exact error** before adding more `unsafe` or restructuring traits.

### Smoke test (required, not optional)

`tests/d1_roundtrip.rs` runs against `wrangler dev`'s local D1 (or a deployed staging DB):

1. POST a tuple via a temporary debug endpoint `/debug/tuple` (gate behind `#[cfg(debug_assertions)]` or a feature flag).
2. GET the tuple back; assert equality.
3. DELETE it.
4. Confirm the GET now 404s.

Add the debug endpoint to the existing Worker router temporarily — Phase 3 (admin GUI) replaces it with proper routes.

### Out of scope (Phase 2)

- Admin GUI (Phase 3)
- Authorization checks (engine work, already done)
- Schema introspection endpoint
- Tuple watch / change feeds
- Multi-tenancy / namespacing (single tenant for now)
- MCP, OpenAPI, AuthZEN endpoints

### Phase 2 process

1. `git status` and `git branch --show-current` → confirm `cloudflare`.
2. `wrangler d1 create plat-authz` → show output and the `database_id` to paste into `wrangler.toml`. (Add a mise task `cf:d1:create`.)
3. Show ported `0001_authz_init.sql` next to a link to the pgauthz original — call out every transform you applied so the user can review.
4. `wrangler d1 migrations apply plat-authz --local` → show output. (Add a mise task `cf:d1:migrations:apply`.)
5. Add `crates/authz-worker/src/d1/` modules. Show diffs incrementally.
6. `mise run worker:check` → show output. **If any error mentions `Send`/`Sync`, stop and show the full error before applying more `unsafe`.**
7. Wire `D1TupleStore` into the Worker entry in place of the stub store. Show that diff.
8. `mise run worker:dev` → curl `/debug/tuple` round-trip (POST, GET, DELETE). Show all three responses.
9. `wrangler d1 migrations apply plat-authz --remote` → `mise run cf:deploy` → curl deployed round-trip. (Add a mise task `cf:d1:migrations:apply:remote`.)
10. Stop.

If any step fails, stop with the exact error. Don't work around `Send`/`Sync` without asking — that's the most likely place to go sideways.

## Phase 3: admin GUI (after Phase 2 is deployed)

> **Plan changed 2026-04-30** — original plan was to hand-roll a maud + Pico + Datastar admin
> behind `/admin/*` on `authz-worker`. Discovered [d1-manager](https://github.com/neverinfamous/d1-manager)
> (v2.6.8, production/stable, April 2026) which is a self-hostable Cloudflare Worker app
> that already does row CRUD, visual schema designer, ER diagrams, query console,
> import/export, and **GitHub SSO via Cloudflare Access**. It replaces ~80% of what we'd
> build. See [`docs/tools.md`](docs/tools.md) for the comparison table. The original
> Datastar/maud spec is preserved in git history in case we ever need to revisit.

### Goal — Phase 3 (revised)

Stand up a GitHub-SSO-protected admin UI for `authz-store` and add a **single
authz-specific endpoint** to `authz-worker` that the engine's `check()` runs through.

Two components:

1. **`authz-admin` Worker** — a self-hosted clone of d1-manager, configured with our D1
   binding pointed at `authz-store`. Behind Cloudflare Access (GitHub OAuth). Lives in a
   sibling Worker so its lifecycle is independent of the engine Worker. Tuple CRUD +
   schema/ERD view come for free.
2. **`POST /check` on `authz-worker`** — JSON in, Allowed/Denied/ConditionRequired out.
   This is the policy check sandbox piece d1-manager doesn't cover (it knows D1, not
   Zanzibar). Kept tiny — no HTML, no maud, no SSE.

### Stack — fixed

- d1-manager from upstream — vendor/clone into `crates/authz-admin/` (or sibling repo, TBD).
- Cloudflare Access — Zero Trust app gating the admin Worker behind GitHub OAuth.
- For `/check`: plain JSON over the existing `worker::fetch` handler in `authz-worker`.
  No new deps.

### Out of scope (this admin phase)

- Hand-rolled HTML/SSE — explicitly **avoided**. d1-manager handles all UI.
- maud, Pico, Datastar, `build.rs` codegen — none of them needed.
- `/admin/tuples`, `/admin/schema` routes on `authz-worker` — d1-manager owns these.
- ignition / Lit (CAD frontend, separate repo, separate session).
- partykit, MCP, OpenAPI integration into admin.

### Phase 3 process

#### Part A — `POST /check` on `authz-worker`

1. Confirm `git branch --show-current` is `cloudflare`.
2. Add a `POST /check` handler in `crates/authz-worker/src/lib.rs`. Body:
   `{ object, relation, subject, context? }`. Return JSON with the engine's
   Allowed/Denied/ConditionRequired result.
3. `mise run worker:check` → `mise run worker:dev` → curl local `/check` with a tuple
   from `vendor/pgauthz/matrix/simple_direct.yaml`.
4. `mise run cf:deploy` → curl the deployed `/check`. Show output.

#### Part B — deploy `authz-admin` (d1-manager)

> **Plan changed 2026-04-30 (after attempting it).** d1-manager turned out to be more
> tangled than the README implied — its `wrangler.toml` ships with `[[r2_buckets]]`,
> `[[durable_objects.bindings]]`, `[ai]`, a `[triggers] crons`, and a `[[routes]]` block
> pointing at the upstream author's domain. Configuring all that in this repo via mise
> tasks means a dependency on internals that drift between releases — exactly the
> "fork without forking" problem.
>
> **More importantly**, d1-manager is by-design **account-level ops infra**: its API
> token is account-scoped and it admins **every** D1 on the account, not just
> `authz-store`. Coupling its deploy to `authz-core` would be a category mistake — it
> belongs in a separate ops repo (e.g. `cf-ops`) or a one-off manual deploy from the
> operator's home dir.

**This repo does not deploy d1-manager. Treat it as a separate concern.**

**Fork created**: https://github.com/joeblew999/d1-manager

This fork is **account-level ops infra** — it admins every D1 on the
`gedw99@gmail.com` account (this project's `authz-store`, plus `analytics-oltp`,
`test-hono-db`, future remy-sport DBs, etc.). It is *not* an authz-core companion.

Worker name = repo name = `d1-manager`. Keeps the URL self-documenting and avoids
bikeshedding.

Minimal `wrangler.toml` diff against upstream:
1. Remove the `[[routes]] pattern = "d1.adamic.tech"` block (their domain, not ours).
2. Set `workers_dev = false` → `true` (so the deploy lands at
   `d1-manager.gedw99.workers.dev` instead of needing a custom domain).
3. Set `database_id` to the value from `wrangler d1 create d1-manager-metadata` (the
   upstream config already uses the right `database_name`).

Everything else stays as upstream ships it: R2 hourly backups, the `BackupDO`
Durable Object, the `[ai]` binding, the cron trigger. R2 storage is sub-cent per
month at our scale; rejecting these features was a false economy.

Steps when ready to deploy:
1. ✅ Fork at `github.com/joeblew999/d1-manager`.
2. In the fork: apply the three-change diff above. Tag as `v0.1`.
3. `wrangler r2 bucket create d1-manager-backups` (one-time CF resource).
4. `wrangler d1 create d1-manager-metadata` (one-time), apply `worker/schema.sql`.
5. Cloudflare Zero Trust: GitHub OAuth IdP + Access Application gating
   `d1-manager.gedw99.workers.dev`. Copy the `POLICY_AUD` audience tag.
6. CF API token (Account → D1: Edit). Copy `ACCOUNT_ID`.
7. `wrangler secret put` for `ACCOUNT_ID`, `API_KEY`, `TEAM_DOMAIN`, `POLICY_AUD`.
8. `npm run build && wrangler deploy` from the fork.
9. From then on it admins every D1 on the account.

Why fork rather than vendor/patch-on-deploy: the upstream `wrangler.toml` ships with
external resource dependencies, and patching them in a downstream task means tracking
those internals. Forking once + pinning is cleaner than sed-on-every-deploy. Cherry-pick
upstream bumps when you want them, not when they break your config.

Until that happens, ops on `authz-store` go through `mise run cf:d1:exec:{local,remote}`
or `wrangler d1 execute` directly. No GUI, but no coupling either.

#### Stop conditions

- **Part A** is the load-bearing piece for downstream consumers — don't skip it.
- **Part B** can be deferred indefinitely; until d1-manager is up we admin via
  `wrangler d1 execute …`. The world doesn't end if Part B isn't done.

Stop at any failure. Show error. Ask before working around it.

## Phase 4: biz-model validation (after Phase 2 D1 is deployed)

This is the empirical "does it actually work end-to-end with the real biz model" check. The earlier analysis (in conversation) was theoretical — we walked the CSVs and concluded they're expressible in the DSL with two known caveats (temporal relations, event-subtype scoping). **Phase 4 proves it by running real checks**, not arguing about it.

### Goal — Phase 4

Load the remy-sport-biz model into the deployed Worker and run a fixture suite of `check()` calls that mirror real scenarios. Every fixture's expected result must match the actual engine response.

### Inputs (from `../remy-sport-biz/`)

- `data/seed/object_types.csv` — 6 types
- `data/seed/relations.csv` — 14 relations (with `derived_from` source-table notes)
- `data/seed/actions.csv` — ~70 actions
- `data/seed/permissions.csv` — (action, relation, [event_type]) triples
- `data/seed/events.csv`, `team_coaches.csv`, `player_teams.csv`, `guardians.csv`, `event_co_organizers.csv`, `users.csv`, etc. — the seed data the tuple-derivation layer reads from

### Pieces to build

1. **`scripts/biz-to-dsl.ts`** (or Rust equivalent) — converts the four model CSVs to authz-core DSL. Idempotent. Output committed for review.
2. **`scripts/biz-derive-tuples.ts`** — reads the seed entity CSVs and emits `(object, relation, subject)` tuple rows. Mirrors the `derived_from` column logic in `relations.csv`. Output is a `.sql` file or a streaming `INSERT` script for D1.
3. **CEL condition for event-subtype scoping** — emitted by `biz-to-dsl.ts` when a permission row has `event_type_code`.
4. **`tests/biz_check.rs`** — the fixture suite. Hand-written cases covering each *category* of rule, not every row.

### Fixture coverage (minimum)

| Scenario | Why |
|---|---|
| OWNER edits own event | Direct relation grant |
| CO_ORGANIZER edits event | Multi-grant union |
| Random user cannot edit event | Negative case |
| OWNER manages divisions on TOURNAMENT | Event-subtype grants |
| OWNER cannot manage divisions on CAMP | Event-subtype denies (CEL condition fires) |
| HEAD_COACH manages roster on own team | Filtered relation (`coach_role_code=HEAD`) |
| ASSISTANT_COACH cannot delete team | Relation precision |
| GUARDIAN signs up minor as player | Conditional on `player.is_minor` |
| Adult cannot use guardian signup path | Same condition, opposite outcome |
| Expired `player_teams` row → no `TEAM_PLAYER` access | Temporal relation handling |
| PUBLIC views event | `[user:*]` wildcard |
| ANY_ORGANIZER creates event | Synthetic platform relation |
| PLATFORM_ADMIN bypasses ownership | Admin override |
| Follower receives notification rights but not edit | Relation isolation |

If any of these fail, that's a real flexibility gap — escalate before patching.

### Two known caveats to specifically prove out

1. **Temporal relations** — `relations.csv` row `TEAM_PLAYER` is "active rows where to_date is empty or future". Two valid implementations:
   - **State-based**: derivation script writes only active tuples, deletes them on expiry.
   - **CEL-based**: write all rows; check uses `now() < to_date` condition.

   Pick **state-based** unless there's a reason not to (cheaper per-check, no clock dependency in the engine). Phase 4 must include a fixture where a tuple is expired and the check correctly denies.

2. **Event-subtype scoping** — `MANAGE_DIVISIONS` only on TOURNAMENT/LEAGUE/SHOWCASE. Implemented as CEL condition `event.type_code in ['TOURNAMENT','LEAGUE','SHOWCASE']`. The Worker must populate `context.event` from D1 before `check()` for those actions. Phase 4 must include a fixture confirming CAMP correctly denies `MANAGE_DIVISIONS`.

### Out of scope (Phase 4)

- Audit logging (decision-002 future ADR)
- Cardinality limits ("max 2 co-organizers") — app-layer concern
- Multi-step workflows (referee approval) — app-layer concern
- Localisation (`name_th`/`name_en`) — never reaches the engine
- Performance / caching strategy — separate phase

### Phase 4 process

1. Confirm Phase 2 D1 store is live and writable.
2. Run `biz-to-dsl.ts` → show generated DSL → load via `PolicyWriter`.
3. Run `biz-derive-tuples.ts` against biz seed CSVs → apply via D1 migration → confirm row count.
4. Implement `tests/biz_check.rs` fixture suite (the table above).
5. `mise run test` → all green, or stop and surface failures.
6. Run the same suite against the **deployed** Worker (not just local). Show output.
7. **If any fixture fails:** stop, do not patch. Discuss whether it's a model translation bug, a derivation bug, or a real engine flexibility gap.
8. Stop. Report which caveats from the earlier theoretical analysis held up and which didn't.

## What NOT to do

**Always:**
- Don't push to `main`. The `cloudflare` branch is the only place work goes until upstream is consulted.
- Don't open a PR upstream until the user explicitly says so (see [Upstream relationship](#upstream-relationship)).
- Don't refactor engine code beyond wasm-compat fixes. If you must, flag it and minimise the diff.
- Don't add deps to the root crate's `Cargo.toml` unless required for wasm. New deps for Worker/D1/admin go in `crates/authz-worker/Cargo.toml`, never the engine crate.
- Don't skip phases — Phase 3 assumes Phase 2 is in place; Phase 2 assumes Phase 1.

**Phase 1 specifically:**
- Don't write a D1 adapter (that's Phase 2).
- Don't add admin routes (that's Phase 3).
- Don't add tests beyond curling `/health`.

## References

- [workers-rs](https://github.com/cloudflare/workers-rs) — the CF Workers Rust SDK.
- Upstream engine: https://github.com/zvectorlabs/authz-core
- pgauthz (reference DB-layer impl, Postgres extension via pgrx): https://github.com/joeblew999/pgauthz
- Consumer project (why we're doing this): [`../remy-sport/`](../remy-sport/) (Cloudflare Worker app) and [`../remy-sport-biz/`](../remy-sport-biz/) (PO source-of-truth: actors, event types, access matrix, decisions).
- Decision driving engine choice: [`../remy-sport-biz/decisions/decision-002-authorisation-engine.md`](../remy-sport-biz/decisions/decision-002-authorisation-engine.md).
