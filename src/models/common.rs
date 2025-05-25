use chrono::NaiveDateTime;
use diesel::{
	FromSqlRow, Selectable,
	deserialize::FromSql,
	expression::AsExpression,
	serialize::{IsNull, ToSql},
	sql_types::{Jsonb, Nullable, VarChar},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum_macros::{Display, EnumString};
use yojo_art_utils::PgString;

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
	PgString,
)]
#[diesel(sql_type = VarChar)]
pub enum SearchableTypes {
	#[strum(serialize = "public")]
	#[serde(rename = "public")]
	/** だれでも */
	Public,
	#[strum(serialize = "followers")]
	#[serde(rename = "followers")]
	/** フォロワーのみ */
	Followers,
	#[strum(serialize = "reacted")]
	#[serde(rename = "reacted")]
	/** 返信かリアクションしたユーザーのみ */
	Reacted,
}
