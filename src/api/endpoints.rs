use std::sync::Arc;

use axum::{
	Router,
	body::Bytes,
	extract::Request,
	response::{IntoResponse, Response},
};
use futures::StreamExt;

use crate::{
	Context, ServerError,
	service::token_service::{PermissionKind, Token},
};

use super::{default_route, notes};

macro_rules! endpoint {
	($app:ident,$i:path,$permission:expr ) => {{
		let s = stringify!($i).replace("::", "/").replace("_", "-");
		let s = &s[..s.len() - 5];
		let r = $app
			.0
			.route(&format!("/api/{}", s), axum::routing::post($i));
		fn check_permission_is_empty<T>(required_permission: &T) -> bool
		where
			T: AsRef<[PermissionKind]>,
		{
			required_permission.as_ref().is_empty()
		}
		if check_permission_is_empty(&$permission) {
			(r, $app.1)
		} else {
			(
				r.layer(axum::middleware::from_fn_with_state(
					(&$permission, $app.1.clone()),
					check_permission_layer,
				)),
				$app.1,
			)
		}
	}};
}

pub fn route<S>(ctx: &Context) -> Router<S> {
	let r = (Router::new(), Arc::new(ctx.clone()));
	//let r = endpoint!(r, drive::files::create::post).layer(axum::extract::DefaultBodyLimit::max(
	//	ctx.config.full_upload_limit as usize,
	//));
	//let r = endpoint!(r, drive::files::delete::post);
	//let r = endpoint!(r, drive::files::multipart::preflight::post);
	//let r = endpoint!(r, drive::files::multipart::partial_upload::post);
	//let r = endpoint!(r, drive::files::multipart::finish_upload::post);
	//let r = endpoint!(r, drive::files::multipart::abort::post);
	let r = endpoint!(r, notes::timeline::post, [PermissionKind::ReadAccount]);
	let r = endpoint!(r, notes::local_timeline::post, []);
	let r = endpoint!(
		r,
		notes::hybrid_timeline::post,
		[PermissionKind::ReadAccount]
	);
	let (r, ctx) = r;
	let r = r.route("/streaming", axum::routing::get(default_route::streaming));
	let r = r.route("/*path", axum::routing::post(default_route::post));
	let r = r.route("/*path", axum::routing::get(default_route::get));
	let r = r.route("/", axum::routing::get(default_route::get));
	r.with_state(ctx)
}
async fn check_permission_layer(
	axum::extract::State((required_permissions, ctx)): axum::extract::State<(
		impl AsRef<[PermissionKind]>,
		Arc<Context>,
	)>,
	request: Request,
	next: axum::middleware::Next,
) -> Result<Response, Response> {
	let request = check_permission(request, ctx, required_permissions.as_ref())
		.await
		.map_err(|e| e.into_response())?;
	Ok(next.run(request).await)
}
async fn check_permission(
	request: Request,
	ctx: Arc<Context>,
	required_permissions: &[PermissionKind],
) -> Result<Request, ServerError> {
	let (parts, body) = request.into_parts();
	let mut stream = body.into_data_stream();
	let mut bb = Vec::new();
	while let Some(x) = stream.next().await {
		let b = x?;
		bb.extend_from_slice(&b[..]);
	}
	let parms: RequestParams = serde_json::from_slice(&bb)?;
	let user_permissions = if let Some(i) = &parms.i {
		ctx.token_service.get_permission(i).await
	} else {
		Default::default()
	};
	for p in required_permissions {
		if !user_permissions.is_allow(*p) {
			return Err(ServerError::new(
				axum::http::StatusCode::FORBIDDEN,
				format!("required_permissions:{:?}", required_permissions),
			));
		}
	}
	let bytes = Bytes::from(bb);
	Ok(Request::from_parts(parts, axum::body::Body::from(bytes)))
}
#[derive(Debug, serde::Deserialize)]
struct RequestParams {
	i: Option<Token>,
}
