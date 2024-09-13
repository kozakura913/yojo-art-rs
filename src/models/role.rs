
use std::collections::HashMap;

use chrono::NaiveDateTime;
use diesel::{deserialize::FromSql, expression::AsExpression, serialize::ToSql, sql_types::{Jsonb, VarChar}, FromSqlRow, Selectable};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

diesel::table! {
	#[sql_name = "role_assignment"]
	role_assignment (id) {
		id -> VarChar,
		userId -> VarChar,
		roleId -> VarChar,
		expiresAt -> Nullable<Timestamp>,
	}
}
#[derive(Debug,Clone,diesel::Insertable,diesel::Queryable,Selectable,diesel::QueryableByName)]
#[diesel(table_name = role_assignment)]
pub struct MiRoleAssignment{
	pub id:String,
	#[diesel(column_name = "userId")]
	pub user_id:String,
	#[diesel(column_name = "roleId")]
	pub role_id:String,
	#[diesel(column_name = "expiresAt")]
	pub expires_at:Option<NaiveDateTime>,
}

diesel::table! {
	#[sql_name = "role"]
	role (id) {
		id -> VarChar,
		updatedAt -> Timestamp,
		lastUsedAt -> Timestamp,
		name -> VarChar,
		description -> VarChar,
		color -> Nullable<VarChar>,
		iconUrl -> Nullable<VarChar>,
		target -> VarChar,
		condFormula -> Jsonb,
		isPublic -> Bool,
		asBadge -> Bool,
		isModerator -> Bool,
		isAdministrator -> Bool,
		isExplorable -> Bool,
		canEditMembersByModerator -> Bool,
		displayOrder -> Int4,
		policies -> Jsonb,
	}
}
#[derive(Debug,Clone,diesel::Insertable,diesel::Queryable,Selectable,diesel::QueryableByName)]
#[diesel(table_name = role)]
pub struct MiRole{
	pub id:String,
	#[diesel(column_name = "updatedAt")]
	pub updated_at:NaiveDateTime,
	#[diesel(column_name = "lastUsedAt")]
	pub last_used_at:NaiveDateTime,
	pub name:String,
	pub description:String,
	pub color:Option<String>,
	#[diesel(column_name = "iconUrl")]
	pub icon_url:Option<String>,
	#[diesel(column_name = "target")]
	pub assign_mode:AssignMode,
	#[diesel(column_name = "condFormula")]
	pub cond_formula:serde_json::Value,
	#[diesel(column_name = "isPublic")]
	pub is_public:bool,
	#[diesel(column_name = "asBadge")]
	pub as_badge:bool,
	#[diesel(column_name = "isModerator")]
	pub is_moderator:bool,
	#[diesel(column_name = "isAdministrator")]
	pub is_administrator:bool,
	#[diesel(column_name = "isExplorable")]
	pub is_explorable:bool,
	#[diesel(column_name = "canEditMembersByModerator")]
	pub can_edit_members_by_moderator:bool,
	#[diesel(column_name = "displayOrder")]
	pub display_order:i32,// UIに表示する際の並び順用(大きいほど先頭)
	pub policies:MiRolePolicies,
}

#[derive(Clone,Default,Debug,Serialize,Deserialize,FromSqlRow, AsExpression)]
#[diesel(sql_type = Jsonb)]
pub struct MiRolePolicies(pub HashMap<String,Policy>);
impl ToSql<Jsonb, diesel::pg::Pg> for MiRolePolicies where serde_json::Value: ToSql<Jsonb, diesel::pg::Pg>{
	fn to_sql<'b>(&'b self,out:&mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result{
		<serde_json::Value as ToSql<Jsonb, diesel::pg::Pg>>::to_sql(&(serde_json::to_value(&self).map_err(|e|Box::new(e))?), &mut out.reborrow())
	}
}
impl<DB: diesel::backend::Backend> FromSql<Jsonb, DB> for MiRolePolicies where serde_json::Value: FromSql<Jsonb, DB>{
	fn from_sql(bytes: DB::RawValue<'_>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let v=<serde_json::Value as FromSql<Jsonb, DB>>::from_sql(bytes)?;
		Ok(serde_json::from_str::<Self>(&v.to_string()).map_err(|e|Box::new(e))?)
	}
}
#[derive(Clone,Serialize,Deserialize,Default,Debug)]
pub struct Policy{
	#[serde(rename = "useDefault")]
	pub use_default: bool,
	pub priority: i32,
	pub value: serde_json::Value,
}
#[derive(Copy,Clone,EnumString,Display,Default,Debug,FromSqlRow, AsExpression)]
#[diesel(sql_type = VarChar)]
pub enum AssignMode{
	#[default]
	#[strum(serialize = "manual")]
	Manual,
	#[strum(serialize = "conditional")]
	Conditional,
	#[strum(serialize = "unknown")]
	Unknown
}
impl ToSql<VarChar, diesel::pg::Pg> for AssignMode where String: ToSql<VarChar, diesel::pg::Pg>{
	fn to_sql<'b>(&'b self,out:&mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result{
		<String as ToSql<VarChar, diesel::pg::Pg>>::to_sql(&self.to_string(), &mut out.reborrow())
	}
}
impl<DB: diesel::backend::Backend> FromSql<VarChar, DB> for AssignMode where String: FromSql<VarChar, DB>{
	fn from_sql(bytes: DB::RawValue<'_>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let v=<String as FromSql<VarChar, DB>>::from_sql(bytes)?;
		use std::str::FromStr;
		Self::from_str(&v).or_else(|_|Ok(Self::Unknown))
	}
}
