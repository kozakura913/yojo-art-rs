use diesel::{ExpressionMethods, QueryDsl, Selectable, SelectableHelper};
use diesel_async::RunQueryDsl;
use crate::DBConnection;

diesel::table! {
	#[sql_name = "user_memo"]
	user_memo (id) {
		id -> VarChar,
		userId -> VarChar,
		targetUserId -> VarChar,
		memo -> VarChar,
	}
}
#[derive(PartialEq, Eq,Debug,Clone,diesel::Insertable,diesel::Queryable,Selectable,diesel::QueryableByName)]
#[diesel(table_name = user_memo)]
pub struct MiUserMemo{
	pub id:String,
	#[diesel(column_name = "userId")]
	pub user_id: String,
	#[diesel(column_name = "targetUserId")]
	pub target_user_id: String,
	pub memo: String,
}
impl MiUserMemo{
	pub async fn load_by_user(con:&mut DBConnection<'_>,user_id:&str,target_user_id:&str)->Option<Self>{
		let res:Self={
			use self::user_memo::dsl::user_memo;
			use self::user_memo::dsl::*;
			user_memo.filter(userId.eq(user_id)).filter(targetUserId.eq(target_user_id)).select(Self::as_select()).first(con).await.map_err(|e|{
				eprintln!("{:?}",e);
			})
		}.ok()?;
		Some(res)
	}
}
