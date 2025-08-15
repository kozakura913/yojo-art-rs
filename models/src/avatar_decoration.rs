use chrono::NaiveDateTime;
use diesel::Selectable;
use yojo_art_utils::LoadByIds;

diesel::table! {
	#[sql_name = "avatar_decoration"]
	avatar_decoration (id) {
		id -> VarChar,
		updatedAt -> Nullable<Timestamp>,
		url -> VarChar,
		name -> VarChar,
		description -> VarChar,
		roleIdsThatCanBeUsedThisDecoration -> Array<VarChar>,
		remoteId -> Nullable<VarChar>,
		host -> Nullable<VarChar>,
		rawUrl -> Nullable<VarChar>,
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
	diesel::QueryableByName,LoadByIds
)]
#[pg_table(table_name = "avatar_decoration")]
#[diesel(table_name = avatar_decoration)]
pub struct MiAvatarDecoration {
	pub id: String,
	#[diesel(column_name = "updatedAt")]
	pub updated_at: Option<NaiveDateTime>,
	pub url: String,
	pub name: String,
	pub description: String,
	// TODO: 定期ジョブで存在しなくなったロールIDを除去するようにする
	#[diesel(column_name = "roleIdsThatCanBeUsedThisDecoration")]
	pub role_ids_that_can_be_used_this_decoration: Vec<String>,
	#[diesel(column_name = "remoteId")]
	pub remote_id: Option<String>,
	pub host: Option<String>,
	#[diesel(column_name = "rawUrl")]
	pub raw_url: Option<String>,
}
