use std::collections::HashMap;

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
	let user_id = permission.as_user_id().ok_or("token")?;
	let policies = permission.get_policies(&ctx.role_service).await;
	if !policies.ltl_available {
		return Err(ServerError::new(
			StatusCode::BAD_REQUEST,
			serde_json::json!({
				"message": "Hybrid timeline has been disabled.",
				"code": "STL_DISABLED",
				"id": "620763f4-f621-4533-ab33-0577a1a3c342",
			})
			.to_string(),
		));
	}
	let meta = ctx.meta_service.load(true).await.ok_or("fetch meta")?;
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
	let mut user_cache = HashMap::new();
	let mut hints = TimelineHints::default();
	let notes = if meta.other.enable_fanout_timeline {
		ctx.fanout_timeline_service
			.get_stl(user_id, &mut hints, &opts)
			.await
	} else {
		ctx.timeline_service
			.get_stl(user_id, &mut hints, &opts)
			.await
	}?;
	let mut note_cache = HashMap::new();
	let mut packed_notes = vec![];
	for note in notes {
		let packed_note = ctx
			.note_service
			.pack_detail(
				note,
				Some(user_id),
				&mut user_cache,
				&mut note_cache,
				&mut hints.note_relation_note,
			)
			.await?;
		packed_notes.push(packed_note);
	}
	let mut header = axum::http::header::HeaderMap::new();
	header.insert("Content-Type", "application/json".parse().unwrap());
	Ok((
		StatusCode::OK,
		header,
		serde_json::to_string(&packed_notes)?,
	)
		.into_response())
}
