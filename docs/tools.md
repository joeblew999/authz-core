# Tools we may want to integrate

A scratch list of useful third-party tools we've spotted but haven't wired in
yet. Add a row when you find one. Promote to actual tooling (mise task,
example, dependency) when you decide to use it.

## D1 / database

| Tool | URL | What it gives us | Potential use |
|---|---|---|---|
| d1-manager | https://github.com/neverinfamous/d1-manager | Full self-hostable D1 admin app on Cloudflare Workers. Tuple/row CRUD across any D1 table, **visual schema designer**, **ER diagrams** (Foreign Key Visualizer), query console (SQL/Drizzle/Query Builder), import/export (SQL/JSON/CSV), Drizzle migrations console, **Zero Trust / GitHub SSO** built in. v2.6.8 (April 2026), production/stable. | Strong candidate to **replace most of Phase 3** (tuple CRUD + schema view) and bring **GitHub auth for free** as the ops tool. Self-hosted on its own Worker, points at `authz-store` D1. We then only need to build the authz-specific **policy check sandbox** in our own Worker — much smaller scope than full hand-rolled admin. See "Phase 3 scope reduction" below. |

### Phase 3 scope reduction (proposed)

If we adopt d1-manager:

| What CLAUDE.md Phase 3 plans to build | What d1-manager already does | Verdict |
|---|---|---|
| `/admin/tuples` — CRUD on tuples (maud + Pico + Datastar SSE) | Full row CRUD on any D1 table, with paging/filters/import-export | **Skip — use d1-manager** |
| `/admin/schema` — read-only schema view | Visual schema designer + ER diagram + FK visualizer | **Skip — use d1-manager (better)** |
| `/admin/check` — policy check sandbox | Generic SQL console only — doesn't know about authz-core's `check()` | **Keep — authz-specific, build small** |
| Auth on the admin (deferred per spec) | GitHub SSO + Cloudflare Access | **Free win** |

Net effect: Phase 3 collapses from "vendor datastar SDK + build.rs codegen + maud + SSE
encoder + 4 routes + golden tests" down to "one Worker route that runs `check()` and
returns the result, plus a deploy of d1-manager pointed at `authz-store`."

## Cloudflare general

*(empty — add tools here as found)*

## Authorization / Zanzibar

*(empty — add tools here as found)*

## Testing / CI

*(empty — add tools here as found)*
