use redis::aio::MultiplexedConnection;

use crate::{DataBase, service::note::NoteService};

#[derive(Clone, Debug)]
pub struct TimelineService {
	db: DataBase,
	redis_for_timelines: MultiplexedConnection,
	note_service: NoteService,
}
pub struct TLOptions {
	pub until_id: Option<String>,
	pub since_id: Option<String>,
	pub with_files: bool,
	pub with_renotes: bool,
	pub allow_partial: bool,
	pub with_cats: bool,
	pub limit: u16,
}
impl TimelineService {
	pub fn new(
		db: DataBase,
		redis_for_timelines: MultiplexedConnection,
		note_service: NoteService,
	) -> Self {
		Self {
			db,
			redis_for_timelines,
			note_service,
		}
	}
}
