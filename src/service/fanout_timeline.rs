use std::sync::Arc;

use redis::{AsyncCommands, aio::MultiplexedConnection};

use crate::{DataBase, MisskeyConfig, ServerError, models::note::MiNote};

use super::{
	event::EventService, id_service::IdService, meta::MetaService, role::RoleService,
	user::UserService,
};

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
			redis_for_timelines,
			host,
		}
	}
}
impl FanoutTimelineService {
	pub async fn get_notes(&self, user_id: &String) -> Result<Vec<MiNote>, ServerError> {
		let tl = self
			.redis_for_timelines
			.clone()
			.lrange::<String, Vec<String>>(
				format!("{}:list:homeTimeline:{}", self.host, user_id),
				0,
				-1,
			)
			.await?;
		println!("{:?}", tl);
		Ok(vec![])
	}
}
