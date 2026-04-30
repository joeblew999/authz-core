-- Ported from pgauthz init.sql.
--
-- Verbatim upstream copy lives at: vendor/pgauthz/init.sql
-- Source URL:  https://github.com/zvectorlabs/pgauthz/blob/main/crates/pgauthz/sql/init.sql
--
-- To re-sync after upstream changes:
--   1. mise run cf:d1:upstream:fetch     (overwrites vendor/pgauthz/init.sql)
--   2. git diff vendor/pgauthz/init.sql  (shows what upstream changed)
--   3. Apply the same transforms below to a NEW migration file (0002_*.sql).
--      Don't edit 0001 — D1 migrations are append-only once applied.
--
-- Transforms applied (Postgres -> SQLite/D1):
--   * Dropped CREATE SCHEMA; SQLite has no schemas. Tables prefixed authz_ instead.
--   * TIMESTAMPTZ -> TEXT (ISO-8601, bound from Rust).
--   * JSONB -> TEXT.
--   * Dropped DEFAULT now() — Rust binds created_at on insert.
--   * Dropped INCLUDE (...) on authz_tuple_covering — SQLite has no covering indexes.
--   * ANALYZE authz.tuple -> ANALYZE authz_tuple.

-- revision: global revision tracking
CREATE TABLE authz_revision (
    revision_id TEXT NOT NULL PRIMARY KEY,
    created_at  TEXT NOT NULL
);

-- authorization_policy: global DSL definition
CREATE TABLE authz_authorization_policy (
    id         TEXT NOT NULL PRIMARY KEY,
    definition TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- tuple: relationship storage (object#relation@subject)
CREATE TABLE authz_tuple (
    object_type  TEXT NOT NULL,
    object_id    TEXT NOT NULL,
    relation     TEXT NOT NULL,
    subject_type TEXT NOT NULL,
    subject_id   TEXT NOT NULL,
    condition    TEXT,
    created_at   TEXT NOT NULL,
    PRIMARY KEY (object_type, object_id, relation, subject_type, subject_id)
);

CREATE INDEX authz_tuple_by_subject
    ON authz_tuple (subject_id, subject_type, object_type, object_id, relation);

CREATE INDEX authz_tuple_object_subject
    ON authz_tuple (object_type, relation, subject_type, subject_id);

CREATE INDEX authz_tuple_covering
    ON authz_tuple (object_type, object_id, relation);

CREATE INDEX authz_tuple_filter
    ON authz_tuple (object_type, subject_type, relation);

CREATE INDEX authz_tuple_watch
    ON authz_tuple (object_type, created_at);

-- changelog: for Watch API
CREATE TABLE authz_changelog (
    object_type  TEXT NOT NULL,
    object_id    TEXT NOT NULL,
    relation     TEXT NOT NULL,
    subject_type TEXT NOT NULL,
    subject_id   TEXT NOT NULL,
    operation    TEXT NOT NULL CHECK (operation IN ('write', 'delete')),
    ulid         TEXT NOT NULL,
    created_at   TEXT NOT NULL
);

CREATE INDEX authz_changelog_object_ulid ON authz_changelog (object_type, ulid);
CREATE INDEX authz_changelog_time        ON authz_changelog (created_at);
CREATE INDEX authz_changelog_object_time ON authz_changelog (object_type, created_at);

-- assertion: for assertion tests
CREATE TABLE authz_assertion (
    id         TEXT NOT NULL PRIMARY KEY,
    assertions TEXT NOT NULL,
    created_at TEXT NOT NULL
);

ANALYZE authz_tuple;
ANALYZE authz_changelog;
