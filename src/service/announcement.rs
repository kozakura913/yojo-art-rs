use diesel::BoolExpressionMethods;

use crate::{models::announcement::MiAnnouncement, DataBase};

#[derive(Clone,Debug)]
pub struct AnnouncementService{
	db:DataBase,
}
impl AnnouncementService{
	pub fn new(db:DataBase)->Self{
		Self{
			db
		}
	}
	pub async fn get_unread_announcements(&self,user_id:&str)->Option<Vec<MiAnnouncement>>{
		let mut con=self.db.get().await?;
		use diesel::{ExpressionMethods, QueryDsl};
		use diesel_async::RunQueryDsl;
		use crate::models::announcement_read::announcement_read;
		use crate::models::announcement::announcement::dsl::announcement;
		use crate::models::announcement::announcement::dsl::*;
		let target_ids = announcement_read::dsl::announcement_read
			.filter(announcement_read::dsl::userId.eq(user_id))
			.select(announcement_read::dsl::announcementId);
		let res:Option<Vec<MiAnnouncement>>=announcement
			.filter(isActive.eq(true))
			.filter(silence.eq(false))
			.filter(userId.eq(user_id).or(userId.eq::<Option<String>>(None)))
			.filter(forExistingUsers.eq(false).or(id.gt(user_id)))
			.filter(diesel::dsl::not(id.eq_any(target_ids)))
			.load(&mut con).await.map_err(|e|{
			eprintln!("{:?}",e);
		}).ok();
		res
	}
}
