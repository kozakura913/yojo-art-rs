use std::sync::Arc;

use axum::Router;

use crate::Context;

use super::{default_route, drive, notes};

macro_rules! endpoint {
	($app:ident,$i:path ) => {{
		let s = stringify!($i).replace("::", "/").replace("_", "-");
		let s = &s[..s.len() - 5];
		$app.route(&format!("/api/{}", s), axum::routing::post($i))
	}};
}

pub fn route<S>(ctx: &Context) -> Router<S> {
	let r = Router::new();
	let r = endpoint!(r, drive::files::create::post).layer(axum::extract::DefaultBodyLimit::max(
		ctx.config.full_upload_limit as usize,
	));
	let r = endpoint!(r, drive::files::delete::post);
	let r = endpoint!(r, drive::files::multipart::preflight::post);
	let r = endpoint!(r, drive::files::multipart::partial_upload::post);
	let r = endpoint!(r, drive::files::multipart::finish_upload::post);
	let r = endpoint!(r, drive::files::multipart::abort::post);
	let r = endpoint!(r, notes::timeline::post);
	let r = endpoint!(r, notes::local_timeline::post);
	let r = endpoint!(r, notes::hybrid_timeline::post);

	let r = r.route("/streaming", axum::routing::get(default_route::streaming));
	let r = r.route("/*path", axum::routing::post(default_route::post));
	let r = r.route("/*path", axum::routing::get(default_route::get));
	let r = r.route("/", axum::routing::get(default_route::get));
	r.with_state(Arc::new(ctx.clone()))
}
