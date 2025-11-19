use crate::{DataBase, ServerError, models::announcement::MiAnnouncement};

#[derive(Clone, Debug)]
pub struct AnnouncementService {
	db: DataBase,
}
impl AnnouncementService {
	pub fn new(db: DataBase) -> Self {
		Self { db }
	}
	pub async fn get_unread_announcements(
		&self,
		user_id: &str,
	) -> Result<Vec<MiAnnouncement>, ServerError> {
		let mut con = ServerError::map_err(
			self.db.get_read_only().await,
			"8ef24664-bbd0-45b5-a7d4-d2305afa27c1",
		)?;
		let res = MiAnnouncement::get_unread_announcements(&mut con, user_id).await;
		Ok(ServerError::map_err(
			res,
			"94231d15-4df6-4676-a928-6119014176d3",
		)?)
	}
}
