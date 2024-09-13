use diesel::{allow_tables_to_appear_in_same_query, Selectable};

use super::announcement::announcement;

diesel::table! {
	#[sql_name = "announcement_read"]
	announcement_read (id) {
		id -> VarChar,
		userId -> VarChar,
		announcementId -> VarChar,
	}
}
#[derive(PartialEq,Eq,Debug,Clone,diesel::Insertable,diesel::Queryable,Selectable,diesel::QueryableByName)]
#[diesel(table_name = announcement_read)]
pub struct MiAnnouncementRead{
	pub id:String,
	#[diesel(column_name = "userId")]
	pub user_id:String,
	#[diesel(column_name = "announcementId")]
	pub announcement_id:String,
}
allow_tables_to_appear_in_same_query!(announcement_read, announcement);
