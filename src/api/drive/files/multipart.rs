use axum::Router;

use crate::Context;

mod preflight;
mod partial_upload;
mod finish_upload;
mod abort;

pub fn route(ctx: &Context,app: Router)->Router{
	let arg_tup0=ctx.clone();
	let app=app.route("/api/drive/files/multipart/preflight",axum::routing::post(move|parms|preflight::post(arg_tup0.clone(),parms)));
	let arg_tup0=ctx.clone();
	let app=app.route("/api/drive/files/multipart/partial-upload",axum::routing::post(move|parms,body|partial_upload::post(arg_tup0.clone(),parms,body)));
	let arg_tup0=ctx.clone();
	let app=app.route("/api/drive/files/multipart/finish-upload",axum::routing::post(move|body|finish_upload::post(arg_tup0.clone(),body)));
	let arg_tup0=ctx.clone();
	let app=app.route("/api/drive/files/multipart/abort",axum::routing::post(move|body|abort::post(arg_tup0.clone(),body)));

	app
}
