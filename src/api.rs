use axum::Router;

use crate::Context;

mod default_route;
mod drive;

pub fn route(ctx: &Context,app: Router)->Router{
	let app=drive::route(ctx,app);
	let arg_tup0=ctx.clone();
	let app=app.route("/streaming",axum::routing::get(move|ws,req|default_route::streaming(arg_tup0.clone(),ws,req)));
	let arg_tup0=ctx.clone();
	let app=app.route("/*path",axum::routing::post(move|body|default_route::post(arg_tup0.clone(),body)));
	let arg_tup0=ctx.clone();
	let app=app.route("/*path",axum::routing::get(move|body|default_route::get(arg_tup0.clone(),body)));
	let arg_tup0=ctx.clone();
	let app=app.route("/",axum::routing::get(move|body|default_route::get(arg_tup0.clone(),body)));
	app
}
