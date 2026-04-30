use async_trait::async_trait;
use authz_core::error::AuthzError;
use authz_core::traits::{Tuple, TupleWriter};
use worker::send::SendFuture;

use super::sql;
use super::store::{D1TupleStore, now_iso8601};

fn map_err(e: worker::Error) -> AuthzError {
    AuthzError::Datastore(e.to_string())
}

#[async_trait]
impl TupleWriter for D1TupleStore {
    async fn write_tuples(
        &self,
        writes: &[Tuple],
        deletes: &[Tuple],
    ) -> Result<String, AuthzError> {
        let writes = writes.to_vec();
        let deletes = deletes.to_vec();
        let now = now_iso8601();

        SendFuture::new(async move {
            let mut stmts = Vec::with_capacity(writes.len() + deletes.len());

            for t in &deletes {
                let stmt = self
                    .db
                    .prepare(sql::DELETE_TUPLE)
                    .bind(&[
                        t.object_type.clone().into(),
                        t.object_id.clone().into(),
                        t.relation.clone().into(),
                        t.subject_type.clone().into(),
                        t.subject_id.clone().into(),
                    ])
                    .map_err(map_err)?;
                stmts.push(stmt);
            }

            for t in &writes {
                let condition: worker::wasm_bindgen::JsValue = match &t.condition {
                    Some(c) => c.clone().into(),
                    None => worker::wasm_bindgen::JsValue::NULL,
                };
                let stmt = self
                    .db
                    .prepare(sql::INSERT_TUPLE)
                    .bind(&[
                        t.object_type.clone().into(),
                        t.object_id.clone().into(),
                        t.relation.clone().into(),
                        t.subject_type.clone().into(),
                        t.subject_id.clone().into(),
                        condition,
                        now.clone().into(),
                    ])
                    .map_err(map_err)?;
                stmts.push(stmt);
            }

            if !stmts.is_empty() {
                self.db.batch(stmts).await.map_err(map_err)?;
            }
            Ok("0".to_string())
        })
        .await
    }
}
