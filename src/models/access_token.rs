use chrono::NaiveDateTime;
use diesel::{ExpressionMethods, QueryDsl, Selectable, SelectableHelper};
use diesel_async::RunQueryDsl;

use crate::DBConnection;

diesel::table! {
	#[sql_name = "access_token"]
	access_token (id) {
		id -> VarChar,
		token -> VarChar,
		session -> Nullable<VarChar>,
		hash -> VarChar,
		userId -> VarChar,
		lastUsedAt -> Nullable<Timestamp>,
		appId -> Nullable<VarChar>,
		name -> Nullable<VarChar>,
		description -> Nullable<VarChar>,
		permission -> Array<VarChar>,
		fetched -> Bool,
		iconUrl -> Nullable<VarChar>,
	}
}
#[derive(Debug,Clone,diesel::Insertable,diesel::Queryable,Selectable,diesel::QueryableByName)]
#[diesel(table_name = access_token)]
pub struct MiAccessToken{
	pub id:String,
	pub token:String,
	pub session:Option<String>,
	pub hash:String,
	#[diesel(column_name = "userId")]
	pub user_id:String,
	#[diesel(column_name = "lastUsedAt")]
	pub last_used_at:Option<NaiveDateTime>,
	#[diesel(column_name = "appId")]
	pub app_id:Option<String>,
	pub name:Option<String>,
	pub description:Option<String>,
	pub permission:Vec<String>,
	pub fetched:bool,
	#[diesel(column_name = "iconUrl")]
	pub icon_url:Option<String>,
}

impl MiAccessToken{
	pub async fn load_by_id(con:&mut DBConnection<'_>,token_id:&str)->Option<Self>{
		let res:MiAccessToken={
			use self::access_token::dsl::access_token;
			use self::access_token::dsl::*;
			access_token.filter(token.eq(token_id)).select(MiAccessToken::as_select()).first(con).await.map_err(|e|{
				eprintln!("{:?}",e);
			})
		}.ok()?;
		Some(res)
	}
}
