use axum::{http::StatusCode, response::IntoResponse};
use futures::TryStreamExt;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;
use tokio_util::io::StreamReader;

use crate::{
	Context, UploadSession,
	models::{
		access_token::MiAccessToken,
		drive_file::{self, MiDriveFile},
		user::MiUser,
	},
	service::{
		event::DriveEventType,
		token_service::{Token, TokenPermission},
	},
};

#[derive(Debug, Deserialize)]
pub struct RequestParams {
	i: Token, //トークン必須
	fileId: String,
}
pub async fn post(
	axum::extract::State(ctx): axum::extract::State<std::sync::Arc<Context>>,
	axum::extract::Json(parms): axum::extract::Json<RequestParams>,
) -> axum::response::Response {
	let permission = ctx.token_service.get_permission(&parms.i).await;
	if !permission.is_allow(crate::service::token_service::PermissionKind::WriteDrive) {
		return StatusCode::FORBIDDEN.into_response();
	}
	println!("DELETE {}", parms.fileId);
	// Check if there is a file with the same hash
	use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
	use diesel_async::RunQueryDsl;
	let mut con = if let Some(con) = ctx.raw_db.get_writeable().await {
		con
	} else {
		return StatusCode::INTERNAL_SERVER_ERROR.into_response();
	};
	let user = match permission.into_user(&mut con).await {
		Ok(user) => user,
		Err(e) => return e.into_response(),
	};

	let file: Option<MiDriveFile> = {
		use crate::models::drive_file::drive_file::dsl::drive_file;
		use crate::models::drive_file::drive_file::dsl::*;
		drive_file
			.filter(id.eq(&parms.fileId))
			.select(MiDriveFile::as_select())
			.first(&mut con)
			.await
			.map_err(|e| {
				eprintln!("{}:{} {:?}", file!(), line!(), e);
			})
	}
	.ok();
	let file = match file {
		Some(f) => f,
		None => return StatusCode::BAD_REQUEST.into_response(),
	};
	if !ctx.role_service.is_moderator(&user.id).await && file.user_id != Some(user.id) {
		return StatusCode::FORBIDDEN.into_response();
	}
	if let Some(access_key) = file.access_key.as_ref() {
		let _ = ctx.bucket.delete_object(access_key).await.map_err(|e| {
			eprintln!("{}:{} {:?}", file!(), line!(), e);
		});
	}
	if let Some(thumbnail_access_key) = file.thumbnail_access_key.as_ref() {
		let _ = ctx
			.bucket
			.delete_object(thumbnail_access_key)
			.await
			.map_err(|e| {
				eprintln!("{}:{} {:?}", file!(), line!(), e);
			});
	}
	let deleted: Option<usize> = {
		use crate::models::drive_file::drive_file::dsl::drive_file;
		use crate::models::drive_file::drive_file::dsl::*;
		diesel::delete(drive_file.filter(id.eq(&parms.fileId)))
			.execute(&mut con)
			.await
			.map_err(|e| {
				eprintln!("{}:{} {:?}", file!(), line!(), e);
			})
	}
	.ok();
	//TODO チャート
	if let Some(user_id) = file.user_id.as_ref() {
		let _ = ctx
			.event_service
			.publish_drive_stream(
				user_id,
				Some(DriveEventType::FileDeleted),
				Some(serde_json::Value::String(file.id)),
			)
			.await;
	}
	//TODO モデログ
	match deleted {
		Some(0) => StatusCode::BAD_REQUEST.into_response(),
		Some(1) => StatusCode::NO_CONTENT.into_response(),
		_ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
	}
}
