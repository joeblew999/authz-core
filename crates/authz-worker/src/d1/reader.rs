use async_trait::async_trait;
use authz_core::error::AuthzError;
use authz_core::traits::{Tuple, TupleFilter, TupleReader};
use worker::send::SendFuture;

use super::sql;
use super::store::{D1TupleStore, TupleRow};

fn map_err(e: worker::Error) -> AuthzError {
    AuthzError::Datastore(e.to_string())
}

#[async_trait]
impl TupleReader for D1TupleStore {
    async fn read_tuples(&self, filter: &TupleFilter) -> Result<Vec<Tuple>, AuthzError> {
        let mut sql = String::from(
            "SELECT object_type, object_id, relation, subject_type, subject_id, condition \
             FROM authz_tuple WHERE 1=1",
        );
        let mut binds: Vec<worker::wasm_bindgen::JsValue> = Vec::new();
        if let Some(v) = &filter.object_type {
            sql.push_str(" AND object_type = ?");
            binds.push(v.clone().into());
        }
        if let Some(v) = &filter.object_id {
            sql.push_str(" AND object_id = ?");
            binds.push(v.clone().into());
        }
        if let Some(v) = &filter.relation {
            sql.push_str(" AND relation = ?");
            binds.push(v.clone().into());
        }
        if let Some(v) = &filter.subject_type {
            sql.push_str(" AND subject_type = ?");
            binds.push(v.clone().into());
        }
        if let Some(v) = &filter.subject_id {
            sql.push_str(" AND subject_id = ?");
            binds.push(v.clone().into());
        }

        SendFuture::new(async move {
            let rows = self
                .db
                .prepare(&sql)
                .bind(&binds)
                .map_err(map_err)?
                .all()
                .await
                .map_err(map_err)?
                .results::<TupleRow>()
                .map_err(map_err)?;
            Ok(rows.into_iter().map(Into::into).collect())
        })
        .await
    }

    async fn read_user_tuple(
        &self,
        object_type: &str,
        object_id: &str,
        relation: &str,
        subject_type: &str,
        subject_id: &str,
    ) -> Result<Option<Tuple>, AuthzError> {
        let binds: [worker::wasm_bindgen::JsValue; 5] = [
            object_type.into(),
            object_id.into(),
            relation.into(),
            subject_type.into(),
            subject_id.into(),
        ];
        SendFuture::new(async move {
            let row = self
                .db
                .prepare(sql::SELECT_USER_TUPLE)
                .bind(&binds)
                .map_err(map_err)?
                .first::<TupleRow>(None)
                .await
                .map_err(map_err)?;
            Ok(row.map(Into::into))
        })
        .await
    }

    async fn read_userset_tuples(
        &self,
        object_type: &str,
        object_id: &str,
        relation: &str,
    ) -> Result<Vec<Tuple>, AuthzError> {
        let binds: [worker::wasm_bindgen::JsValue; 3] =
            [object_type.into(), object_id.into(), relation.into()];
        SendFuture::new(async move {
            let rows = self
                .db
                .prepare(sql::SELECT_USERSET)
                .bind(&binds)
                .map_err(map_err)?
                .all()
                .await
                .map_err(map_err)?
                .results::<TupleRow>()
                .map_err(map_err)?;
            Ok(rows.into_iter().map(Into::into).collect())
        })
        .await
    }

    async fn read_starting_with_user(
        &self,
        subject_type: &str,
        subject_id: &str,
    ) -> Result<Vec<Tuple>, AuthzError> {
        let binds: [worker::wasm_bindgen::JsValue; 2] = [subject_type.into(), subject_id.into()];
        SendFuture::new(async move {
            let rows = self
                .db
                .prepare(sql::SELECT_STARTING_WITH_USER)
                .bind(&binds)
                .map_err(map_err)?
                .all()
                .await
                .map_err(map_err)?
                .results::<TupleRow>()
                .map_err(map_err)?;
            Ok(rows.into_iter().map(Into::into).collect())
        })
        .await
    }

    async fn read_user_tuple_batch(
        &self,
        object_type: &str,
        object_id: &str,
        relations: &[String],
        subject_type: &str,
        subject_id: &str,
    ) -> Result<Option<Tuple>, AuthzError> {
        for relation in relations {
            if let Some(t) = self
                .read_user_tuple(object_type, object_id, relation, subject_type, subject_id)
                .await?
            {
                return Ok(Some(t));
            }
        }
        Ok(None)
    }
}
