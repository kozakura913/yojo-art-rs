use diesel::Selectable;
use diesel_async::RunQueryDsl;
use crate::DBConnection;

diesel::table! {
	#[sql_name = "note_reaction"]
	note_reaction (id) {
		id -> VarChar,
		userId -> VarChar,
		noteId -> VarChar,
		reaction -> VarChar,
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
#[diesel(table_name = note_reaction)]
pub struct MiNoteReaction {
	pub id: String,
	#[diesel(column_name = "userId")]
	pub user_id: String,
	#[diesel(column_name = "noteId")]
	pub note_id: String,
	pub reaction: String,
}
impl MiNoteReaction{
	pub async fn insert_into(&self,con:&mut DBConnection<'_>)-> Result<(), diesel::result::Error> {
		use self::note_reaction::dsl::note_reaction;
		diesel::insert_into(note_reaction)
			.values(self)
			.execute(con)
			.await?;
		Ok(())
	}
}