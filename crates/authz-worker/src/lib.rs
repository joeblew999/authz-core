mod d1;

use authz_core::traits::{Tuple, TupleFilter, TupleReader, TupleWriter};
use serde::Deserialize;
use worker::*;

use d1::D1TupleStore;

#[derive(Debug, Deserialize)]
struct TupleBody {
    object_type: String,
    object_id: String,
    relation: String,
    subject_type: String,
    subject_id: String,
    #[serde(default)]
    condition: Option<String>,
}

impl From<TupleBody> for Tuple {
    fn from(b: TupleBody) -> Self {
        Tuple {
            object_type: b.object_type,
            object_id: b.object_id,
            relation: b.relation,
            subject_type: b.subject_type,
            subject_id: b.subject_id,
            condition: b.condition,
        }
    }
}

fn tuple_to_json(t: &Tuple) -> serde_json::Value {
    serde_json::json!({
        "object_type":  t.object_type,
        "object_id":    t.object_id,
        "relation":     t.relation,
        "subject_type": t.subject_type,
        "subject_id":   t.subject_id,
        "condition":    t.condition,
    })
}

fn filter_from_query(url: &Url) -> (TupleFilter, [String; 5]) {
    let mut f = TupleFilter::default();
    let mut keys = [
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ];
    for (k, v) in url.query_pairs() {
        match k.as_ref() {
            "object_type" => {
                keys[0] = v.to_string();
                f.object_type = Some(v.into_owned());
            }
            "object_id" => {
                keys[1] = v.to_string();
                f.object_id = Some(v.into_owned());
            }
            "relation" => {
                keys[2] = v.to_string();
                f.relation = Some(v.into_owned());
            }
            "subject_type" => {
                keys[3] = v.to_string();
                f.subject_type = Some(v.into_owned());
            }
            "subject_id" => {
                keys[4] = v.to_string();
                f.subject_id = Some(v.into_owned());
            }
            _ => {}
        }
    }
    (f, keys)
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    let url = req.url()?;
    let path = url.path().to_string();
    let method = req.method();

    if path == "/health" {
        return Response::from_json(&serde_json::json!({ "ok": true }));
    }

    if path == "/debug/tuple" {
        let store = D1TupleStore::new(env.d1("DB")?);
        return debug_tuple(req, &store).await;
    }

    let _ = method;
    Response::error("Not Found", 404)
}

async fn debug_tuple(mut req: Request, store: &D1TupleStore) -> Result<Response> {
    match req.method() {
        Method::Post => {
            let body: TupleBody = req.json().await?;
            let t: Tuple = body.into();
            store
                .write_tuples(std::slice::from_ref(&t), &[])
                .await
                .map_err(|e| worker::Error::RustError(e.to_string()))?;
            Response::from_json(&tuple_to_json(&t))
        }
        Method::Get => {
            let url = req.url()?;
            let (_, k) = filter_from_query(&url);
            let row = store
                .read_user_tuple(&k[0], &k[1], &k[2], &k[3], &k[4])
                .await
                .map_err(|e| worker::Error::RustError(e.to_string()))?;
            match row {
                Some(t) => Response::from_json(&tuple_to_json(&t)),
                None => Response::error("Not Found", 404),
            }
        }
        Method::Delete => {
            let url = req.url()?;
            let (_, k) = filter_from_query(&url);
            let t = Tuple {
                object_type: k[0].clone(),
                object_id: k[1].clone(),
                relation: k[2].clone(),
                subject_type: k[3].clone(),
                subject_id: k[4].clone(),
                condition: None,
            };
            store
                .write_tuples(&[], std::slice::from_ref(&t))
                .await
                .map_err(|e| worker::Error::RustError(e.to_string()))?;
            Response::from_json(&serde_json::json!({ "deleted": true }))
        }
        _ => Response::error("Method Not Allowed", 405),
    }
}
