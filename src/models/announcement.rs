use chrono::NaiveDateTime;
use diesel::{deserialize::{FromSql, FromSqlRow}, expression::AsExpression, serialize::ToSql, sql_types::VarChar, Selectable};
use strum_macros::{Display, EnumString};

diesel::table! {
	#[sql_name = "announcement"]
	announcement (id) {
		id -> VarChar,
		updatedAt -> Nullable<Timestamp>,
		text -> VarChar,
		title -> VarChar,
		imageUrl -> Nullable<VarChar>,
		icon -> VarChar,
		display -> VarChar,
		needConfirmationToRead -> Bool,
		isActive -> Bool,
		forExistingUsers -> Bool,
		silence -> Bool,
		userId -> Nullable<VarChar>,
	}
}
#[derive(PartialEq,Eq,Debug,Clone,diesel::Insertable,diesel::Queryable,Selectable,diesel::QueryableByName)]
#[diesel(table_name = announcement)]
pub struct MiAnnouncement{
	pub id:String,
	#[diesel(column_name = "updatedAt")]
	pub updated_at:Option<NaiveDateTime>,
	pub text:String,
	pub title:String,
	#[diesel(column_name = "imageUrl")]
	pub image_url:Option<String>,
	pub icon:IconType,
	pub display:DisplayType,
	#[diesel(column_name = "needConfirmationToRead")]
	pub need_confirmation_to_read:bool,
	#[diesel(column_name = "isActive")]
	pub is_active:bool,//default: true
	#[diesel(column_name = "forExistingUsers")]
	pub for_existing_users:bool,
	pub silence:bool,
	#[diesel(column_name = "userId")]
	pub user_id:Option<String>,
}
#[derive(PartialEq,Eq,Copy,Clone,EnumString,Display,Default,Debug,FromSqlRow, AsExpression)]
#[diesel(sql_type = VarChar)]
pub enum IconType{
	#[default]
	#[strum(serialize = "info")]
	Info,
	#[strum(serialize = "warning")]
	Warning,
	#[strum(serialize = "error")]
	Error,
	#[strum(serialize = "success")]
	Success,
}
impl ToSql<VarChar, diesel::pg::Pg> for IconType where String: ToSql<VarChar, diesel::pg::Pg>{
	fn to_sql<'b>(&'b self,out:&mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result{
		<String as ToSql<VarChar, diesel::pg::Pg>>::to_sql(&self.to_string(), &mut out.reborrow())
	}
}
impl<DB: diesel::backend::Backend> FromSql<VarChar, DB> for IconType where String: FromSql<VarChar, DB>{
	fn from_sql(bytes: DB::RawValue<'_>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let v=<String as FromSql<VarChar, DB>>::from_sql(bytes)?;
		use std::str::FromStr;
		Self::from_str(&v).or_else(|_|Ok(Self::Info))
	}
}
#[derive(PartialEq,Eq,Copy,Clone,EnumString,Display,Default,Debug,FromSqlRow, AsExpression)]
#[diesel(sql_type = VarChar)]
pub enum DisplayType{
	#[default]
	#[strum(serialize = "normal")]
	Normal,// normal ... お知らせページ掲載
	#[strum(serialize = "banner")]
	Banner,// banner ... お知らせページ掲載 + バナー表示
	#[strum(serialize = "dialog")]
	Dialog,// dialog ... お知らせページ掲載 + ダイアログ表示
}
impl ToSql<VarChar, diesel::pg::Pg> for DisplayType where String: ToSql<VarChar, diesel::pg::Pg>{
	fn to_sql<'b>(&'b self,out:&mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result{
		<String as ToSql<VarChar, diesel::pg::Pg>>::to_sql(&self.to_string(), &mut out.reborrow())
	}
}
impl<DB: diesel::backend::Backend> FromSql<VarChar, DB> for DisplayType where String: FromSql<VarChar, DB>{
	fn from_sql(bytes: DB::RawValue<'_>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let v=<String as FromSql<VarChar, DB>>::from_sql(bytes)?;
		use std::str::FromStr;
		Self::from_str(&v).or_else(|_|Ok(Self::Normal))
	}
}
