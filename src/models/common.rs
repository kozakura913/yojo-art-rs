use diesel::{FromSqlRow, deserialize::FromSql, expression::AsExpression, serialize::ToSql};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use yojo_art_utils::PgEnum;

#[derive(
	Copy,
	Clone,
	EnumString,
	PartialEq,
	Eq,
	Display,
	Debug,
	FromSqlRow,
	AsExpression,
	Serialize,
	Deserialize,
	PgEnum,
)]
#[diesel(sql_type = NoteSearchableType)]
#[pg_type(sql_type = "NoteSearchableType")]
pub enum NoteSearchableBy {
	#[strum(serialize = "public")]
	#[serde(rename = "public")]
	/** だれでも */
	Public,
	#[strum(serialize = "followersAndReacted")]
	#[serde(rename = "followersAndReacted")]
	/** フォロワーのみ */
	FollowersAndReacted,
	#[strum(serialize = "reactedOnly")]
	#[serde(rename = "reactedOnly")]
	/** 返信かリアクションしたユーザーのみ */
	ReactedOnly,
	#[strum(serialize = "private")]
	#[serde(rename = "private")]
	/** 投稿者のみ */
	Private,
}

#[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
#[diesel(postgres_type(name = "note_searchableby_enum"))]
pub struct NoteSearchableType;

#[derive(
	Copy,
	Clone,
	EnumString,
	PartialEq,
	Eq,
	Display,
	Debug,
	FromSqlRow,
	AsExpression,
	Serialize,
	Deserialize,
	PgEnum,
)]
#[diesel(sql_type = UserSearchableType)]
#[pg_type(sql_type = "UserSearchableType")]
pub enum UserSearchableBy {
	#[strum(serialize = "public")]
	#[serde(rename = "public")]
	/** だれでも */
	Public,
	#[strum(serialize = "followersAndReacted")]
	#[serde(rename = "followersAndReacted")]
	/** フォロワーのみ */
	FollowersAndReacted,
	#[strum(serialize = "reactedOnly")]
	#[serde(rename = "reactedOnly")]
	/** 返信かリアクションしたユーザーのみ */
	ReactedOnly,
	#[strum(serialize = "private")]
	#[serde(rename = "private")]
	/** 投稿者のみ */
	Private,
}

#[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
#[diesel(postgres_type(name = "user_searchableby_enum"))]
pub struct UserSearchableType;
