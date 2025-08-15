use chrono::NaiveDateTime;
use diesel::Selectable;
use yojo_art_utils::LoadByIds;

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
#[derive(
	Debug, Clone, diesel::Insertable, diesel::Queryable, Selectable, diesel::QueryableByName,LoadByIds
)]
#[pg_table(table_name = "access_token")]
#[diesel(table_name = access_token)]
pub struct MiAccessToken {
	pub id: String,
	pub token: String,
	pub session: Option<String>,
	pub hash: String,
	#[diesel(column_name = "userId")]
	pub user_id: String,
	#[diesel(column_name = "lastUsedAt")]
	pub last_used_at: Option<NaiveDateTime>,
	#[diesel(column_name = "appId")]
	pub app_id: Option<String>,
	pub name: Option<String>,
	pub description: Option<String>,
	pub permission: Vec<String>,
	pub fetched: bool,
	#[diesel(column_name = "iconUrl")]
	pub icon_url: Option<String>,
}
