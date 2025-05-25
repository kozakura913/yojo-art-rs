use chrono::NaiveDateTime;
use diesel::{
	ExpressionMethods, FromSqlRow, QueryDsl, Selectable, SelectableHelper,
	deserialize::FromSql,
	expression::AsExpression,
	serialize::{IsNull, ToSql},
	sql_types::{Jsonb, Nullable, VarChar},
};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use yojo_art_utils::PgString;

diesel::table! {
	#[sql_name = "emoji"]
	emoji (id) {
		id -> VarChar,
		updatedAt -> Nullable<Timestamp>,
		name -> VarChar,
		host -> Nullable<VarChar>,
		category -> Nullable<VarChar>,
		originalUrl -> VarChar,
		publicUrl -> VarChar,
		uri -> Nullable<VarChar>,
		r#type -> Nullable<VarChar>,
		aliases -> Array<VarChar>,
		license -> Nullable<VarChar>,
		localOnly -> Bool,
		isSensitive -> Bool,
		usageInfo -> Nullable<VarChar>,
		description -> Nullable<VarChar>,
		author -> Nullable<VarChar>,
		copyPermission -> Nullable<VarChar>,
		isBasedOn -> Nullable<VarChar>,
		importFrom -> Nullable<VarChar>,
		roleIdsThatCanBeUsedThisEmojiAsReaction -> Array<VarChar>,
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
#[diesel(table_name = emoji)]
pub struct MiEmoji {
	pub id: String,
	#[diesel(column_name = "updatedAt")]
	pub updated_at: Option<NaiveDateTime>,
	pub name: String,
	pub host: Option<String>,
	pub category: Option<String>,
	#[diesel(column_name = "originalUrl")]
	pub original_url: String,
	#[diesel(column_name = "publicUrl")]
	pub public_url: String,
	pub uri: Option<String>,
	// publicUrlの方のtypeが入る
	#[diesel(column_name = "type")]
	pub image_type: Option<String>,
	pub aliases: Vec<String>,
	pub license: Option<String>,
	#[diesel(column_name = "localOnly")]
	pub local_only: bool,
	#[diesel(column_name = "isSensitive")]
	pub is_sensitive: bool,
	#[diesel(column_name = "usageInfo")]
	pub usage_info: Option<String>,
	pub description: Option<String>,
	pub author: Option<String>,
	#[diesel(column_name = "copyPermission")]
	pub copy_permission: Option<EmojiCopyPermissions>,
	#[diesel(column_name = "isBasedOn")]
	pub is_based_on: Option<String>,
	#[diesel(column_name = "importFrom")]
	pub import_from: Option<String>,
	// TODO: 定期ジョブで存在しなくなったロールIDを除去するようにする
	#[diesel(column_name = "roleIdsThatCanBeUsedThisEmojiAsReaction")]
	pub role_ids_that_can_be_used_this_emoji_as_reaction: Vec<String>,
}
#[derive(
	Copy,
	Clone,
	PartialEq,
	Eq,
	EnumString,
	Display,
	Default,
	Debug,
	FromSqlRow,
	AsExpression,
	Serialize,
	Deserialize,
	PgString,
)]
#[diesel(sql_type = VarChar)]
pub enum EmojiCopyPermissions {
	#[default]
	#[strum(serialize = "allow")]
	#[serde(rename = "allow")]
	Allow,
	#[strum(serialize = "deny")]
	#[serde(rename = "deny")]
	Deny,
	#[strum(serialize = "conditional")]
	#[serde(rename = "conditional")]
	Conditional,
}
