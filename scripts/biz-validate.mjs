#!/usr/bin/env node
// Phase 4 v1 — biz-model validation against the deployed authz-worker.
//
// Reads vendored remy-sport-biz seed CSVs, generates an authz-core DSL model
// covering EVENT + PLATFORM (the most-exercised types), seeds the matching
// tuples in D1 via /debug/tuple, then runs hand-crafted check fixtures from
// CLAUDE.md's Phase 4 table.
//
// Usage:  node scripts/biz-validate.mjs [worker-url]
// Env:    WORKER_URL  (overrides positional arg)
//
// Out: per-assertion PASS/FAIL with a NOTE on each failure indicating
//      whether it's an engine gap, a model translation gap, or expected.
//
// Exit 0 if all assertions pass, 1 otherwise.

import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const SEED_DIR = "vendor/remy-sport-biz/data/seed";
const URL = process.env.WORKER_URL ||
  process.argv[2] ||
  "https://authz-worker.gedw99.workers.dev";

// ── tiny CSV parser (these CSVs have no quoted/embedded commas) ──────────────
function loadCsv(name) {
  const path = join(SEED_DIR, name);
  const lines = readFileSync(path, "utf8").trim().split("\n");
  const header = lines[0].split(",");
  return lines.slice(1).map(line => {
    const cells = line.split(",");
    return Object.fromEntries(header.map((h, i) => [h, cells[i]]));
  });
}

// ── load model CSVs ──────────────────────────────────────────────────────────
const objectTypes = loadCsv("object_types.csv");
const relations   = loadCsv("relations.csv");
const permissions = loadCsv("permissions.csv");
// Entity seed
const events            = loadCsv("events.csv");
const eventCoOrganizers = loadCsv("event_co_organizers.csv");
const users             = loadCsv("users.csv");

// ── generate DSL (v1: EVENT + PLATFORM only) ─────────────────────────────────
//
// Translation rules:
//   * lowercase all type/relation/action codes (DSL is lowercase by convention)
//   * Permissions on a non-PLATFORM action that reference a PLATFORM relation
//     bridge via a "platform" link relation: define platform: [platform];
//     then `platform->permission`.
//   * event_type_code conditional permissions are EMITTED UNCONDITIONALLY in
//     v1 — we'll see them fail in fixture 6 (CAMP cannot manage_divisions),
//     which is the engine-gap signal.

const lc = s => (s || "").toLowerCase();
const PLATFORM = "platform";

function dslForType(typeCode) {
  const t = lc(typeCode);
  const rels = relations.filter(r => r.object_type_code === typeCode);
  // Permissions where action.object_type matches THIS type
  // (we infer action.object_type from actions.csv — but a simpler approach:
  //  derive from permissions.csv joined with the type of relations)
  // For v1 we focus on EVENT + PLATFORM, so we hardcode which actions belong
  // to which type via a small map below.

  let s = `type ${t} {\n`;
  s += `  relations\n`;
  for (const r of rels) {
    // PUBLIC accepts wildcard; use union syntax. Engine syntax is [user | user:*].
    if (r.code === "PUBLIC") {
      s += `    define public: [user | user:*]\n`;
    } else {
      s += `    define ${lc(r.code)}: [user]\n`;
    }
  }
  // PLATFORM bridge for non-platform types
  if (typeCode !== "PLATFORM") {
    s += `    define ${PLATFORM}: [platform]\n`;
  }

  // Permissions — dedupe by (action, expr).
  const perms = permsForType(typeCode);
  if (Object.keys(perms).length > 0) {
    s += `  permissions\n`;
    for (const [action, relList] of Object.entries(perms)) {
      const seen = new Set();
      const exprs = [];
      for (const rel of relList) {
        const relRow = relations.find(r => r.code === rel);
        if (!relRow) continue;
        let expr;
        if (relRow.object_type_code === typeCode) expr = lc(rel);
        else if (relRow.object_type_code === "PLATFORM") expr = `${PLATFORM}->${lc(rel)}`;
        else continue;   // cross-type non-platform: skip in v1
        if (!seen.has(expr)) {
          seen.add(expr);
          exprs.push(expr);
        }
      }
      if (exprs.length === 0) continue;
      s += `    define ${lc(action)} = ${exprs.join(" + ")}\n`;
    }
  }
  s += `}\n`;
  return s;
}

// Group permissions.csv by action_code → list of relations.
// For v1, decide which type "owns" the action by majority-vote of its relations.
function permsForType(typeCode) {
  const out = {};
  for (const p of permissions) {
    const rel = relations.find(r => r.code === p.relation_code);
    if (!rel) continue;
    // Heuristic: action belongs to type if its relation is on this type,
    // or (for non-platform actions referencing PLATFORM relations only) we
    // attach to the action's "primary" type. We use actions.csv's
    // object_type_code below, simpler.
  }
  // Read actions.csv directly to know each action's owning type
  const actions = loadCsv("actions.csv");
  const actionOwnerType = Object.fromEntries(actions.map(a => [a.code, a.object_type_code]));
  for (const p of permissions) {
    if (actionOwnerType[p.action_code] !== typeCode) continue;
    if (!out[p.action_code]) out[p.action_code] = [];
    out[p.action_code].push(p.relation_code);
  }
  return out;
}

const MODEL = `
type user {}
${dslForType("PLATFORM")}
${dslForType("EVENT")}
`.trim();

// ── derive tuples (v1: EVENT direct relations + PLATFORM admin/organizer) ────

const tuples = [];
const platformId = "p";   // singleton platform object id
// event.owner from events.organizer_user_id
for (const e of events) {
  tuples.push({
    object_type: "event", object_id: e.id, relation: "owner",
    subject_type: "user", subject_id: e.organizer_user_id,
  });
  tuples.push({
    object_type: "event", object_id: e.id, relation: "platform",
    subject_type: "platform", subject_id: platformId,
  });
}
// event.co_organizer from event_co_organizers
for (const co of eventCoOrganizers) {
  tuples.push({
    object_type: "event", object_id: co.event_id, relation: "co_organizer",
    subject_type: "user", subject_id: co.user_id,
  });
}
// platform.platform_admin from users where role_code=ADMIN
for (const u of users.filter(u => u.role_code === "ADMIN")) {
  tuples.push({
    object_type: "platform", object_id: platformId, relation: "platform_admin",
    subject_type: "user", subject_id: u.id,
  });
}
// platform.any_organizer from users where role_code=ORGANIZER
for (const u of users.filter(u => u.role_code === "ORGANIZER")) {
  tuples.push({
    object_type: "platform", object_id: platformId, relation: "any_organizer",
    subject_type: "user", subject_id: u.id,
  });
}

// ── HTTP helpers ─────────────────────────────────────────────────────────────

const seeded = new Set();   // for cleanup

async function seedTuple(t) {
  const r = await fetch(`${URL}/debug/tuple`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(t),
  });
  if (!r.ok) throw new Error(`seed failed ${r.status} ${await r.text()}`);
  seeded.add(JSON.stringify(t));
}

async function deleteTuple(t) {
  const qs = new URLSearchParams(t).toString();
  await fetch(`${URL}/debug/tuple?${qs}`, { method: "DELETE" });
}

async function check(model, object_type, object_id, relation, subject_type, subject_id) {
  const r = await fetch(`${URL}/check`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      model, object_type, object_id, relation, subject_type, subject_id,
    }),
  });
  const j = await r.json();
  return j.result;
}

async function cleanup() {
  for (const ts of seeded) {
    await deleteTuple(JSON.parse(ts));
  }
}
process.on("exit", () => { /* fire-and-forget cleanup runs below */ });

// ── fixtures (CLAUDE.md Phase 4 table — v1 subset) ───────────────────────────

const fixtures = [
  {
    name: "owner_edits_own_event",
    args: ["event", "evt_001", "edit_event", "user", "usr_org_001"],
    expect: "Allowed",
    note: "direct OWNER relation, simplest case",
  },
  {
    name: "co_organizer_edits_event",
    args: ["event", "evt_001", "edit_event", "user", "usr_org_002"],
    expect: "Allowed",
    note: "multi-grant union: OWNER + CO_ORGANIZER",
  },
  {
    name: "random_user_cannot_edit",
    args: ["event", "evt_001", "edit_event", "user", "usr_player_001"],
    expect: "Denied",
    note: "negative case — no relation, no admin",
  },
  {
    name: "platform_admin_bypasses_ownership",
    args: ["event", "evt_001", "edit_event", "user", "usr_admin_001"],
    expect: "Allowed",
    note: "PLATFORM relation traversal: event->platform->platform_admin",
  },
  {
    name: "owner_manages_divisions_on_tournament",
    args: ["event", "evt_001", "manage_divisions", "user", "usr_org_001"],
    expect: "Allowed",
    note: "evt_001 IS a TOURNAMENT — passes regardless of CEL",
  },
  {
    name: "owner_cannot_manage_divisions_on_camp",
    args: ["event", "evt_003", "manage_divisions", "user", "usr_org_003"],
    expect: "Denied",
    note: "evt_003 is CAMP. Without CEL/subtype-split this WILL fail — exposes gap.",
  },
  {
    name: "any_organizer_creates_event",
    args: ["platform", "p", "create_event", "user", "usr_org_001"],
    expect: "Allowed",
    note: "synthetic platform relation — every ORGANIZER user gets ANY_ORGANIZER",
  },
];

// ── run ──────────────────────────────────────────────────────────────────────

console.log(`== biz-validate v1 (against ${URL}) ==\n`);
console.log(`-- generated DSL (${MODEL.split("\n").length} lines) --`);
console.log(MODEL);
console.log(`\n-- seeding ${tuples.length} tuples --`);
for (const t of tuples) await seedTuple(t);

let pass = 0, fail = 0;
const failures = [];
for (const f of fixtures) {
  const got = await check(MODEL, ...f.args);
  if (got === f.expect) {
    console.log(`  PASS  ${f.name}`);
    pass++;
  } else {
    console.log(`  FAIL  ${f.name}  expected ${f.expect}, got ${got}`);
    console.log(`        NOTE: ${f.note}`);
    fail++;
    failures.push(f);
  }
}

console.log(`\n== Cleanup ==`);
await cleanup();
console.log(`removed ${seeded.size} tuples`);

console.log(`\n== Result: ${pass} passed, ${fail} failed ==`);
if (failures.length > 0) {
  console.log(`\n== Failure summary (engine gaps or model-translation gaps) ==`);
  for (const f of failures) {
    console.log(`  - ${f.name}: ${f.note}`);
  }
}
process.exit(fail === 0 ? 0 : 1);
