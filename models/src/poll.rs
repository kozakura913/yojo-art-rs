use chrono::NaiveDateTime;
use diesel::Selectable;

use super::note::NoteVisibility;

diesel::table! {
	#[sql_name = "poll"]
	poll (noteId) {
		noteId -> VarChar,
		expiresAt -> Nullable<Timestamp>,
		multiple -> Bool,
		choices -> Array<VarChar>,
		votes -> Array<Int4>,
		noteVisibility -> crate::note::NoteVisibilityType,
		userId -> VarChar,
		userHost -> Nullable<VarChar>,
		channelId -> Nullable<VarChar>,
	}
}
#[derive(
	PartialEq,
	Eq,
	Debug,
	Clone,
	diesel::Insertable,
	diesel::Queryable,
	Selectable,
	diesel::QueryableByName,
)]
#[diesel(table_name = poll)]
pub struct MiPoll {
	#[diesel(column_name = "noteId")]
	pub note_id: String,
	#[diesel(column_name = "expiresAt")]
	pub expires_at: Option<NaiveDateTime>,
	pub multiple: bool,
	pub choices: Vec<String>,
	pub votes: Vec<i32>,
	#[diesel(column_name = "noteVisibility")]
	pub note_visibility: NoteVisibility,
	#[diesel(column_name = "userId")]
	pub user_id: String,
	#[diesel(column_name = "userHost")]
	pub user_host: Option<String>,
	#[diesel(column_name = "channelId")]
	pub channel_id: Option<String>,
}
