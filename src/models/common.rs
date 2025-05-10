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
impl ToSql<VarChar, diesel::pg::Pg> for SearchableTypes
where
	String: ToSql<VarChar, diesel::pg::Pg>,
{
	fn to_sql<'b>(
		&'b self,
		out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
	) -> diesel::serialize::Result {
		<String as ToSql<VarChar, diesel::pg::Pg>>::to_sql(&self.to_string(), &mut out.reborrow())
	}
}
impl<DB: diesel::backend::Backend> FromSql<VarChar, DB> for SearchableTypes
where
	String: FromSql<VarChar, DB>,
{
	fn from_sql(bytes: DB::RawValue<'_>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let v = <String as FromSql<VarChar, DB>>::from_sql(bytes)?;
		use std::str::FromStr;
		Ok(Self::from_str(&v).or_else(|e| Err(Box::new(e)))?)
	}
}
