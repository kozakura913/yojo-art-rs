use crate::DBConnection;
use diesel::pg::sql_types::Jsonb;
use diesel::{
	ExpressionMethods, QueryDsl, Selectable, SelectableHelper,
	deserialize::{FromSql, FromSqlRow},
	expression::AsExpression,
	serialize::ToSql,
	sql_types::VarChar,
};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use yojo_art_utils::{PgJson, PgString};

diesel::table! {
	#[sql_name = "user_profile"]
	user_profile (userId) {
		userId -> VarChar,
		alwaysMarkNsfw -> Bool,
		autoSensitive -> Bool,
		followingVisibility -> VarChar,
		followersVisibility -> VarChar,
		mutedInstances -> Jsonb,
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
#[diesel(table_name = user_profile)]
pub struct MiUserProfile {
	#[diesel(column_name = "userId")]
	pub user_id: String,
	#[diesel(column_name = "alwaysMarkNsfw")]
	pub always_mark_nsfw: bool,
	#[diesel(column_name = "autoSensitive")]
	pub auto_sensitive: bool,
	#[diesel(column_name = "followingVisibility")]
	pub following_visibility: Visibility,
	#[diesel(column_name = "followersVisibility")]
	pub followers_visibility: Visibility,
	#[diesel(column_name = "mutedInstances")]
	pub muted_instances: MutedInstances,
}

#[derive(
	PartialEq, Eq, Clone, Default, Debug, Serialize, Deserialize, FromSqlRow, AsExpression, PgJson,
)]
#[diesel(sql_type = Jsonb)]
pub struct MutedInstances(Vec<String>);
impl MutedInstances {
	pub fn into_inner(self) -> Vec<String> {
		self.0
	}
}

#[derive(
	PartialEq,
	Eq,
	Copy,
	Clone,
	EnumString,
	Display,
	Default,
	Debug,
	FromSqlRow,
	AsExpression,
	PgString,
)]
#[diesel(sql_type = VarChar)]
pub enum Visibility {
	#[default]
	#[strum(serialize = "public")]
	Public,
	#[strum(serialize = "followers")]
	Followers,
	#[strum(serialize = "private")]
	Private,
}

impl MiUserProfile {
	pub async fn load_by_user(con: &mut DBConnection<'_>, user_id: &str) -> Option<Self> {
		let res: MiUserProfile = {
			use self::user_profile::dsl::user_profile;
			use self::user_profile::dsl::*;
			user_profile
				.filter(userId.eq(user_id))
				.select(MiUserProfile::as_select())
				.first(con)
				.await
				.map_err(|e| {
					eprintln!("{}:{} {:?}", file!(), line!(), e);
				})
		}
		.ok()?;
		Some(res)
	}
}
