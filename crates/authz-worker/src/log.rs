//! Structured JSON logging for Workers Observability.
//!
//! Emits one JSON object per call to console.log. The CF dashboard log
//! explorer indexes top-level keys for filtering — keep them flat.

use worker::console_log;

pub fn event(kind: &str, fields: serde_json::Value) {
    let mut payload = serde_json::json!({ "event": kind });
    if let serde_json::Value::Object(map) = fields {
        if let serde_json::Value::Object(target) = &mut payload {
            for (k, v) in map {
                target.insert(k, v);
            }
        }
    }
    console_log!("{}", payload);
}
