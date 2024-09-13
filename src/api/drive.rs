use axum::Router;

use crate::Context;

mod files;

pub fn route(ctx: &Context,app: Router)->Router{
	files::route(ctx,app)
}
