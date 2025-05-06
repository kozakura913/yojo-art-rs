use axum::{http::StatusCode, response::IntoResponse};

use crate::Context;

pub async fn post(
	axum::extract::State(ctx): axum::extract::State<std::sync::Arc<Context>>,
	request: axum::extract::Request,
) -> axum::response::Response {
	ctx.meta_service.fetch();
	//TODO 良い感じ
	(StatusCode::OK, "[]".to_owned()).into_response()
}
