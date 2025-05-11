use axum::{http::StatusCode, response::IntoResponse};
use redis::AsyncCommands;
use serde::Deserialize;

use crate::{Context, ServerError, service::token_service::Token};

#[derive(Debug, Deserialize)]
pub struct RequestParams {
	i: Token, //トークン必須
	#[serde(rename = "allowPartial")]
	allow_partial: Option<bool>,
	limit: Option<u32>,
	#[serde(rename = "withCats")]
	with_cats: Option<bool>,
	#[serde(rename = "withRenotes")]
	with_renotes: Option<bool>,
}
pub async fn post(
	axum::extract::State(ctx): axum::extract::State<std::sync::Arc<Context>>,
	axum::extract::Json(parms): axum::extract::Json<RequestParams>,
) -> Result<axum::response::Response, ServerError> {
	let permission = ctx.token_service.get_permission(&parms.i).await;
	let meta = ctx.meta_service.load(true).await.ok_or("fetch meta")?;
	let user_id = permission.as_user_id().await.ok_or("token")?;
	if meta.other.enable_fanout_timeline {
		let notes = ctx.fanout_timeline_service.home_tl(user_id).await?;
		return Ok((StatusCode::OK, serde_json::to_string(&notes)?).into_response());
	}
	//TODO 良い感じ
	Ok((StatusCode::OK, "[]".to_owned()).into_response())
}
