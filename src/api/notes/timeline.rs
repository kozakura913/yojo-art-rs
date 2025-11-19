use axum::{http::StatusCode, response::IntoResponse};
use serde::Deserialize;

use crate::{
	Context, ServerError,
	service::{
		timeline::{TLOptions, TimelineHints},
		token_service::Token,
	},
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
	let meta = ServerError::map_opt(
		ctx.meta_service.load(true).await,
		"fetch meta",
		"5e0c43f9-03d3-4ad1-9db6-a1d4fbc83fb0",
	)?;
	let user_id = ServerError::map_opt(
		permission.as_user_id(),
		"token",
		"eb726f3f-cfeb-4b1c-9b88-451a5b164a56",
	)?;
	let opts = TLOptions {
		since_id: parms.since_id,
		until_id: parms.until_id,
		with_files: parms.with_files.unwrap_or(false),
		with_renotes: parms.with_renotes.unwrap_or(false),
		allow_partial: parms.allow_partial.unwrap_or(true),
		with_cats: parms.with_cats.unwrap_or(false),
		limit: parms.limit.unwrap_or(10),
		with_replies: false,
	};
	let mut hints = TimelineHints::default();
	let notes = if meta.other.enable_fanout_timeline {
		ctx.fanout_timeline_service
			.get_htl(user_id, &mut hints, &opts)
			.await
	} else {
		ctx.timeline_service
			.get_htl(user_id, &mut hints, &opts)
			.await
	}?;
	let packed_notes = ctx
		.note_service
		.pack_detail_many(notes.into_iter(), Some(user_id), &hints.note_relation_note)
		.await;
	let mut header = axum::http::header::HeaderMap::new();
	header.insert("Content-Type", "application/json".parse().unwrap());
	Ok((
		StatusCode::OK,
		header,
		ServerError::map_err(
			serde_json::to_string(&packed_notes),
			"a67fe80a-da5d-4cf9-a409-180a2b4b05a0",
		)?,
	)
		.into_response())
}
