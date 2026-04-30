# vendor/

Vendored copies of external sources we depend on for schema, fixtures, and seed
data. Everything here is **read-only** and refreshed via mise tasks — never edit
by hand.

## Why vendor?

Two reasons:

1. **Drift visibility.** When upstream changes their schema or fixtures, the
   refresh task overwrites the vendored copy. `git diff vendor/<source>/` then
   shows you exactly what changed so you can re-apply ports or update tests.
2. **Reproducible builds.** CI doesn't reach out to the network for source-of-
   truth files. The vendored copy in this repo is what gets used.

## Sources

| Path                          | Source                                                                                                  | Refresh task                            | Used in    |
|-------------------------------|---------------------------------------------------------------------------------------------------------|-----------------------------------------|------------|
| `pgauthz/init.sql`            | https://github.com/zvectorlabs/pgauthz/blob/main/crates/pgauthz/sql/init.sql                            | `mise run cf:d1:upstream:fetch`         | Phase 2 (D1 schema port → `migrations/0001_authz_init.sql`) |
| `pgauthz/matrix/*.yaml`       | https://github.com/zvectorlabs/pgauthz/tree/main/crates/pgauthz/tests/matrix                            | `mise run cf:d1:upstream:fetch:fixtures`| Phase 4 (engine semantics fixtures)                          |
| `remy-sport-biz/data/seed/*`  | https://github.com/joeblew999/remy-sport-biz/tree/main/data/seed *(not yet vendored — added in Phase 4)*| TBD                                     | Phase 4 (biz model + seed for the real check fixture suite)  |

## Refresh workflow

When upstream changes:

```sh
mise run cf:d1:upstream:fetch          # refresh schema
mise run cf:d1:upstream:fetch:fixtures # refresh matrix YAMLs
git diff vendor/                       # see what moved
```

For the schema, drift means writing a **new** migration file
(`migrations/0002_*.sql`) — never edit `0001`, since D1 migrations are
append-only once applied.

For the matrix YAMLs and biz seed CSVs, drift means updating fixture-driven
tests — the engine itself doesn't change.
