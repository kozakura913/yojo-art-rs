use diesel::Selectable;

diesel::table! {
	#[sql_name = "poll_vote"]
	poll_vote (id) {
		id -> VarChar,
		userId -> VarChar,
		noteId -> VarChar,
		choice -> Int4,
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
#[diesel(table_name = poll_vote)]
pub struct MiPollVote {
	pub id: String,
	#[diesel(column_name = "userId")]
	pub user_id: String,
	#[diesel(column_name = "noteId")]
	pub note_id: String,
	pub choice: i32,
}
