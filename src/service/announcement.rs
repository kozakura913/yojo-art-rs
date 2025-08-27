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
		let mut con = self.db.get_read_only().await?;
		Ok(MiAnnouncement::get_unread_announcements(&mut con, user_id).await?)
	}
}
