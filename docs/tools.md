# Tools wired in

Third-party tools we evaluated and adopted. Removed from "potential" once
deployed and proven; the canonical record of what's actually live is in
[CLOUDFLARE.md](../CLOUDFLARE.md) under "Sibling Workers".

## Adopted

| Tool | Where | What it gives us |
|---|---|---|
| [d1-manager](https://github.com/neverinfamous/d1-manager) (forked at [joeblew999/d1-manager](https://github.com/joeblew999/d1-manager)) | https://d1-manager.gedw99.workers.dev | Account-wide D1 admin GUI behind GitHub SSO via CF Access. Replaced Phase 3's plan to hand-roll a maud + Pico + Datastar admin on `authz-worker`. |
| [kv-manager](https://github.com/dreamcatcher-tv/kv-manager) (forked at [joeblew999/kv-manager](https://github.com/joeblew999/kv-manager)) | https://kv-manager.gedw99.workers.dev | Account-wide KV admin GUI, same gating. |

Both forks share the operator-tooling pattern in [joeblew999/.github](https://github.com/joeblew999/.github) `mise-tasks/` (cf:provision-d1-r2, cf:access-setup, cf:secrets-put-mapped, prove:*) — any future ops tool fork inherits the same shape.

## Considered, not adopted

*(empty — add rejections here when you decide against something, with a one-line reason)*

## Watching (might be useful)

*(empty — add tools here when you spot one but haven't decided)*
