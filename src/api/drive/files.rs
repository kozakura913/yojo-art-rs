use axum::Router;

use crate::Context;

mod create;
mod multipart;

pub fn route(ctx: &Context,app: Router)->Router{
	let ctx0=ctx.clone();
	let app=app.route("/api/drive/files/create",axum::routing::post(move|multipart|create::post(ctx0.clone(),multipart)))
		.layer(axum::extract::DefaultBodyLimit::max(ctx.config.full_upload_limit as usize));
	multipart::route(ctx,app)
}
