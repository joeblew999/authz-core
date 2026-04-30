use std::collections::HashMap;

use authz_core::core_resolver::CoreResolver;
use authz_core::model_parser::parse_dsl;
use authz_core::policy_provider::StaticPolicyProvider;
use authz_core::resolver::{CheckResolver, CheckResult, ResolveCheckRequest};
use authz_core::traits::Tuple;
use authz_core::type_system::TypeSystem;
use serde::Deserialize;
use worker::{Method, Request, Response, Result};

use crate::d1::D1TupleStore;

#[derive(Debug, Deserialize)]
pub struct CheckBody {
    /// DSL source for the authorization model.
    /// In Phase 4 this will move to D1 and the body will reference it by id;
    /// for the Phase 3 sandbox we accept it inline.
    model: String,

    object_type: String,
    object_id: String,
    relation: String,
    subject_type: String,
    subject_id: String,

    #[serde(default)]
    context: HashMap<String, serde_json::Value>,

    #[serde(default)]
    contextual_tuples: Vec<TupleBody>,
}

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

fn check_result_to_json(r: &CheckResult) -> serde_json::Value {
    match r {
        CheckResult::Allowed => serde_json::json!({ "result": "Allowed" }),
        CheckResult::Denied => serde_json::json!({ "result": "Denied" }),
        CheckResult::ConditionRequired(params) => serde_json::json!({
            "result": { "ConditionRequired": params }
        }),
    }
}

fn bad_request(reason: &str) -> Result<Response> {
    let mut r = Response::from_json(&serde_json::json!({ "error": reason }))?;
    r = r.with_status(400);
    Ok(r)
}

pub async fn handle(mut req: Request, store: D1TupleStore) -> Result<Response> {
    if req.method() != Method::Post {
        return Response::error("Method Not Allowed", 405);
    }

    let body: CheckBody = match req.json().await {
        Ok(b) => b,
        Err(e) => return bad_request(&format!("invalid JSON body: {e}")),
    };

    let model = match parse_dsl(&body.model) {
        Ok(m) => m,
        Err(e) => return bad_request(&format!("model parse error: {e}")),
    };

    let type_system = TypeSystem::new(model);
    let provider = StaticPolicyProvider::new(type_system);
    let resolver = CoreResolver::new(store, provider);

    let mut request = ResolveCheckRequest::new(
        body.object_type,
        body.object_id,
        body.relation,
        body.subject_type,
        body.subject_id,
    );
    request.context = body.context;
    request.contextual_tuples = body.contextual_tuples.into_iter().map(Into::into).collect();

    let log_subject = format!(
        "{}:{}#{}@{}:{}",
        request.object_type,
        request.object_id,
        request.relation,
        request.subject_type,
        request.subject_id,
    );

    match resolver.resolve_check(request).await {
        Ok(result) => {
            let outcome = match &result {
                CheckResult::Allowed => "Allowed",
                CheckResult::Denied => "Denied",
                CheckResult::ConditionRequired(_) => "ConditionRequired",
            };
            crate::log::event(
                "check",
                serde_json::json!({
                    "tuple": log_subject,
                    "outcome": outcome,
                }),
            );
            Response::from_json(&check_result_to_json(&result))
        }
        Err(e) => {
            crate::log::event(
                "check_error",
                serde_json::json!({
                    "tuple": log_subject,
                    "error": format!("{e}"),
                }),
            );
            let mut r = Response::from_json(&serde_json::json!({
                "error": format!("{e}")
            }))?;
            r = r.with_status(500);
            Ok(r)
        }
    }
}
