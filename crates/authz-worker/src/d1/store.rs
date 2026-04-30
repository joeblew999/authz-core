use std::sync::Arc;

use authz_core::traits::Tuple;
use serde::Deserialize;
use worker::D1Database;

#[derive(Clone)]
pub struct D1TupleStore {
    pub(crate) db: Arc<D1Database>,
}

impl D1TupleStore {
    pub fn new(db: D1Database) -> Self {
        Self { db: Arc::new(db) }
    }
}

// Workers isolates are single-threaded, but authz-core's traits require Send + Sync.
// SendFuture wrapping at every call site keeps this sound.
unsafe impl Send for D1TupleStore {}
unsafe impl Sync for D1TupleStore {}

#[derive(Debug, Deserialize)]
pub(crate) struct TupleRow {
    pub object_type: String,
    pub object_id: String,
    pub relation: String,
    pub subject_type: String,
    pub subject_id: String,
    pub condition: Option<String>,
}

impl From<TupleRow> for Tuple {
    fn from(r: TupleRow) -> Self {
        Tuple {
            object_type: r.object_type,
            object_id: r.object_id,
            relation: r.relation,
            subject_type: r.subject_type,
            subject_id: r.subject_id,
            condition: r.condition,
        }
    }
}

pub(crate) fn now_iso8601() -> String {
    js_sys::Date::new_0().to_iso_string().into()
}
