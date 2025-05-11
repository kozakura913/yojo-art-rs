use std::{collections::HashMap, sync::Arc};

use redis::{AsyncCommands, aio::MultiplexedConnection};

use crate::{DBConnection, DataBase, MisskeyConfig, ServerError, models::note::MiNote};

use super::{
	event::EventService,
	id_service::IdService,
	meta::MetaService,
	note::{NoteService, PackedNote},
	role::RoleService,
	user::UserService,
};
pub enum FanoutTimelineName<'a> {
	Home(&'a String),
	Local,
}
impl FanoutTimelineName<'_> {
	fn to_name(&self, host: impl AsRef<str>) -> String {
		match self {
			FanoutTimelineName::Home(user_id) => {
				format!("{}:list:homeTimeline:{}", host.as_ref(), user_id)
			}
			FanoutTimelineName::Local => todo!(),
		}
	}
}
#[derive(Clone, Debug)]
pub struct FanoutTimelineService {
	config: Arc<MisskeyConfig>,
	db: DataBase,
	meta_service: MetaService,
	role_service: RoleService,
	id_service: IdService,
	user_service: UserService,
	event_service: EventService,
	redis_for_timelines: MultiplexedConnection,
	host: String,
	note_service: NoteService,
}
impl FanoutTimelineService {
	pub fn new(
		config: Arc<MisskeyConfig>,
		db: DataBase,
		meta_service: MetaService,
		role_service: RoleService,
		id_service: IdService,
		user_service: UserService,
		event_service: EventService,
		note_service: NoteService,
		redis_for_timelines: MultiplexedConnection,
		host: String,
	) -> Self {
		Self {
			config,
			db,
			meta_service,
			role_service,
			id_service,
			user_service,
			event_service,
			note_service,
			redis_for_timelines,
			host,
		}
	}
	pub async fn home_tl(&self, user_id: &String) -> Result<Vec<PackedNote>, ServerError> {
		let mut con = self.db.get().await.ok_or("db error")?;
		let mut user_cache = HashMap::new();
		let notes = self
			.get_notes(&mut con, &FanoutTimelineName::Home(user_id))
			.await?;
		let mut packed_notes = vec![];
		for note in notes {
			let packed_note = self
				.note_service
				.pack_detail(&mut con, note, user_id, &mut user_cache)
				.await?;
			packed_notes.push(packed_note);
		}
		Ok(packed_notes)
	}
	pub async fn get_notes(
		&self,
		con: &mut DBConnection<'_>,
		timeline: &FanoutTimelineName<'_>,
	) -> Result<Vec<MiNote>, ServerError> {
		let tl = self
			.redis_for_timelines
			.clone()
			.lrange::<String, Vec<String>>(timeline.to_name(&self.host), 0, -1)
			.await?;
		println!("{:?}", tl);
		use diesel::ExpressionMethods;
		use diesel::{QueryDsl, SelectableHelper};
		use diesel_async::RunQueryDsl;
		use tokio::sync::RwLock;

		use crate::{DataBase, models::note::MiNote};

		let notes: Vec<MiNote> = {
			use crate::models::note::note::dsl::note;
			use crate::models::note::note::dsl::*;
			note.filter(id.eq_any(&tl))
				.select(MiNote::as_select())
				.load(con)
				.await
				.map_err(|e| {
					eprintln!("{:?}", e);
				})
		}?;
		Ok(notes)
	}
}
