use diesel::{ExpressionMethods, QueryDsl, Selectable, SelectableHelper};
use diesel_async::RunQueryDsl;
use crate::DBConnection;

diesel::table! {
	#[sql_name = "user_note_pining"]
	user_note_pining (id) {
		id -> VarChar,
		userId -> VarChar,
		noteId -> VarChar,
	}
}
#[derive(PartialEq, Eq,Debug,Clone,diesel::Insertable,diesel::Queryable,Selectable,diesel::QueryableByName)]
#[diesel(table_name = user_note_pining)]
pub struct MiUserNotePining{
	pub id:String,
	#[diesel(column_name = "userId")]
	pub user_id: String,
	#[diesel(column_name = "noteId")]
	pub note_id: String,
}
impl MiUserNotePining{
	pub async fn load_by_user(con:&mut DBConnection<'_>,user_id:&str)->Option<Vec<Self>>{
		let res:Vec<Self>={
			use self::user_note_pining::dsl::user_note_pining;
			use self::user_note_pining::dsl::*;
			user_note_pining.filter(userId.eq(user_id)).order(id.desc()).select(Self::as_select()).load(con).await.map_err(|e|{
				eprintln!("{:?}",e);
			})
		}.ok()?;
		Some(res)
	}
}
