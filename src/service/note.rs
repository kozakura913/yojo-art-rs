use std::{borrow::Cow, str::FromStr, sync::Arc};

use chrono::SecondsFormat;
use serde::{Deserialize, Serialize};

use crate::{
	DBConnection, DataBase, MisskeyConfig, ServerError,
	models::{
		self,
		drive_file::{FileProperties, MiDriveFile},
		drive_folder::MiDriveFolder,
		note::MiNote,
		user::MiUser,
		user_profile::MiUserProfile,
	},
	service::{
		self,
		event::{DriveEventType, MainEventType},
	},
};

use super::{
	event::EventService, id_service::IdService, meta::MetaService, role::RoleService,
	user::UserService,
};
#[derive(Clone, Debug)]
pub struct NoteService {
	config: Arc<MisskeyConfig>,
	db: DataBase,
	meta_service: MetaService,
	role_service: RoleService,
	id_service: IdService,
	user_service: UserService,
	event_service: EventService,
}

impl NoteService {
	pub fn new(
		config: Arc<MisskeyConfig>,
		db: DataBase,
		meta_service: MetaService,
		role_service: RoleService,
		id_service: IdService,
		user_service: UserService,
		event_service: EventService,
	) -> Self {
		Self {
			config,
			db,
			meta_service,
			role_service,
			id_service,
			user_service,
			event_service,
		}
	}
	pub async fn pack(&self, note: MiNote) -> Result<PackedNote, ServerError> {
		Ok(PackedNote {
			createdAt: self
				.id_service
				.parse(&note.id)
				.ok_or("")?
				.to_rfc3339_opts(SecondsFormat::Millis, true),
			updatedAt: note
				.updated_at
				.as_ref()
				.map(|time| time.and_utc().to_rfc3339_opts(SecondsFormat::Millis, true)),
			updated_at_history: note.updated_at_history.as_ref().map(|v| {
				use std::iter::Iterator;
				v.iter()
					.map(|time| time.and_utc().to_rfc3339_opts(SecondsFormat::Millis, true))
					.collect::<Vec<String>>()
			}),
			userId: note.user_id,
			id: note.id,
		})
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackedNote {
	id: String,
	createdAt: String,
	updatedAt: Option<String>,
	updated_at_history: Option<Vec<String>>,
	//noteEditHistory
	//deleteAt
	//user:
	userId: String,
}
