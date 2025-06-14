use axum::{http::StatusCode, response::IntoResponse};
use redis::AsyncCommands;
use serde::Deserialize;

use crate::{
	Context, ServerError,
	service::{timeline::TLOptions, token_service::Token},
};

#[derive(Debug, Deserialize)]
pub struct RequestParams {
	i: Token, //トークン必須
	#[serde(rename = "allowPartial")]
	allow_partial: Option<bool>,
	limit: Option<u16>,
	#[serde(rename = "withCats")]
	with_cats: Option<bool>,
	#[serde(rename = "withRenotes")]
	with_renotes: Option<bool>,
	#[serde(rename = "withFiles")]
	with_files: Option<bool>,
	#[serde(rename = "untilId")]
	until_id: Option<String>,
	#[serde(rename = "sinceId")]
	since_id: Option<String>,
}
pub async fn post(
	axum::extract::State(ctx): axum::extract::State<std::sync::Arc<Context>>,
	axum::extract::Json(parms): axum::extract::Json<RequestParams>,
) -> Result<axum::response::Response, ServerError> {
	let permission = ctx.token_service.get_permission(&parms.i).await;
	let meta = ctx.meta_service.load(true).await.ok_or("fetch meta")?;
	let user_id = permission.as_user_id().await.ok_or("token")?;
	let opts = TLOptions {
		since_id: parms.since_id,
		until_id: parms.until_id,
		with_files: parms.with_files.unwrap_or(false),
		with_renotes: parms.with_renotes.unwrap_or(false),
		allow_partial: parms.allow_partial.unwrap_or(true),
		with_cats: parms.with_cats.unwrap_or(false),
		limit: parms.limit.unwrap_or(10),
	};
	let notes = if meta.other.enable_fanout_timeline {
		ctx.fanout_timeline_service.home_tl(user_id, &opts).await
	} else {
		ctx.timeline_service.home_tl(user_id, &opts).await
	}?;
	let mut header = axum::http::header::HeaderMap::new();
	header.insert("Content-Type", "application/json".parse().unwrap());
	Ok((StatusCode::OK, header, serde_json::to_string(&notes)?).into_response())
}
