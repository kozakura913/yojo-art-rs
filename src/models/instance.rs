use chrono::NaiveDateTime;
use diesel::{
	FromSqlRow, QueryDsl, Selectable,
	deserialize::FromSql,
	expression::AsExpression,
	serialize::ToSql,
	sql_types::{Jsonb, Nullable, VarChar},
};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use yojo_art_utils::PgString;

diesel::table! {
	#[sql_name = "instance"]
	instance (id) {
		id -> VarChar,
		firstRetrievedAt -> Timestamp,
		host -> VarChar,
		usersCount -> Int4,
		notesCount -> Int4,
		followingCount -> Int4,
		followersCount -> Int4,
		latestRequestReceivedAt -> Nullable<Timestamp>,
		isNotResponding -> Bool,
		notRespondingSince -> Nullable<Timestamp>,
		suspensionState -> VarChar,
		softwareName -> Nullable<VarChar>,
		softwareVersion -> Nullable<VarChar>,
		openRegistrations -> Nullable<Bool>,
		name -> Nullable<VarChar>,
		description -> Nullable<VarChar>,
		maintainerName -> Nullable<VarChar>,
		maintainerEmail -> Nullable<VarChar>,
		iconUrl -> Nullable<VarChar>,
		faviconUrl -> Nullable<VarChar>,
		themeColor -> Nullable<VarChar>,
		infoUpdatedAt -> Nullable<Timestamp>,
		moderationNote -> VarChar,
		reversiVersion -> Nullable<VarChar>,
		quarantineLimited -> Bool,
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
	Serialize,
	Deserialize,
)]
#[diesel(table_name = instance)]
pub struct MiInstance {
	pub id: String,
	/**
	 * このインスタンスを捕捉した日時
	 */
	#[diesel(column_name = "firstRetrievedAt")]
	pub first_retrieved_at: NaiveDateTime,
	/**
	 * ホスト
	 */
	pub host: String,
	/**
	 * インスタンスのユーザー数
	 */
	#[diesel(column_name = "usersCount")]
	pub users_count: i32,
	/**
	 * インスタンスの投稿数
	 */
	#[diesel(column_name = "notesCount")]
	pub notes_count: i32,
	/**
	 * このインスタンスのユーザーからフォローされている、自インスタンスのユーザーの数
	 */
	#[diesel(column_name = "followingCount")]
	pub followingCount: i32,
	/**
	 * このインスタンスのユーザーをフォローしている、自インスタンスのユーザーの数
	 */
	#[diesel(column_name = "followersCount")]
	pub followersCount: i32,
	/**
	 * 直近のリクエスト受信日時
	 */
	#[diesel(column_name = "latestRequestReceivedAt")]
	pub latestRequestReceivedAt: Option<NaiveDateTime>,
	/**
	 * このインスタンスと不通かどうか
	 */
	#[diesel(column_name = "isNotResponding")]
	pub isNotResponding: bool,
	/**
	 * このインスタンスと不通になった日時
	 */
	#[diesel(column_name = "notRespondingSince")]
	pub notRespondingSince: Option<NaiveDateTime>,
	/**
	 * このインスタンスへの配信状態
	 */
	#[diesel(column_name = "suspensionState")]
	pub suspensionState: SuspensionState,
	#[diesel(column_name = "softwareName")]
	pub softwareName: Option<String>,
	#[diesel(column_name = "softwareVersion")]
	pub softwareVersion: Option<String>,
	#[diesel(column_name = "openRegistrations")]
	pub openRegistrations: Option<bool>,
	pub name: Option<String>,
	pub description: Option<String>,
	#[diesel(column_name = "maintainerName")]
	pub maintainerName: Option<String>,
	#[diesel(column_name = "maintainerEmail")]
	pub maintainerEmail: Option<String>,
	#[diesel(column_name = "iconUrl")]
	pub iconUrl: Option<String>,
	#[diesel(column_name = "faviconUrl")]
	pub faviconUrl: Option<String>,
	#[diesel(column_name = "themeColor")]
	pub themeColor: Option<String>,
	#[diesel(column_name = "infoUpdatedAt")]
	pub infoUpdatedAt: Option<NaiveDateTime>,
	#[diesel(column_name = "moderationNote")]
	pub moderationNote: String,
	#[diesel(column_name = "reversiVersion")]
	pub reversiVersion: Option<String>,
	/**
	 * このインスタンスへの配送制限
	 */
	#[diesel(column_name = "quarantineLimited")]
	pub quarantineLimited: bool,
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
pub enum SuspensionState {
	#[default]
	#[strum(serialize = "none")]
	#[serde(rename = "none")]
	None,
	#[strum(serialize = "manuallySuspended")]
	#[serde(rename = "manuallySuspended")]
	ManuallySuspended,
	#[strum(serialize = "goneSuspended")]
	#[serde(rename = "goneSuspended")]
	GoneSuspended,
	#[strum(serialize = "autoSuspendedForNotResponding")]
	#[serde(rename = "autoSuspendedForNotResponding")]
	AutoSuspendedForNotResponding,
}
