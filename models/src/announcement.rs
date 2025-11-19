use chrono::NaiveDateTime;
use diesel::{
	Selectable,
	deserialize::{FromSql, FromSqlRow},
	expression::AsExpression,
	serialize::ToSql,
	sql_types::VarChar,
};
use strum_macros::{Display, EnumString};
use yojo_art_utils::PgString;

use crate::DBConnection;

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
#[diesel(table_name = announcement)]
pub struct MiAnnouncement {
	pub id: String,
	#[diesel(column_name = "updatedAt")]
	pub updated_at: Option<NaiveDateTime>,
	pub text: String,
	pub title: String,
	#[diesel(column_name = "imageUrl")]
	pub image_url: Option<String>,
	pub icon: IconType,
	pub display: DisplayType,
	#[diesel(column_name = "needConfirmationToRead")]
	pub need_confirmation_to_read: bool,
	#[diesel(column_name = "isActive")]
	pub is_active: bool, //default: true
	#[diesel(column_name = "forExistingUsers")]
	pub for_existing_users: bool,
	pub silence: bool,
	#[diesel(column_name = "userId")]
	pub user_id: Option<String>,
}
impl MiAnnouncement{
	pub async fn get_unread_announcements(
		con:&mut DBConnection<'_>,
		user_id: &str,
	) -> Result<Vec<Self>, crate::Error> {
		use diesel::BoolExpressionMethods;
		use self::announcement::dsl::announcement;
		use self::announcement::dsl::*;
		use crate::announcement_read::announcement_read;
		use diesel::{ExpressionMethods, QueryDsl};
		use diesel_async::RunQueryDsl;
		let target_ids = announcement_read::dsl::announcement_read
			.filter(announcement_read::dsl::userId.eq(user_id))
			.select(announcement_read::dsl::announcementId);
		let res: Vec<MiAnnouncement> = announcement
			.filter(isActive.eq(true))
			.filter(silence.eq(false))
			.filter(userId.eq(user_id).or(userId.eq::<Option<String>>(None)))
			.filter(forExistingUsers.eq(false).or(id.gt(user_id)))
			.filter(diesel::dsl::not(id.eq_any(target_ids)))
			.load(con)
			.await?;
		Ok(res)
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
pub enum IconType {
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
pub enum DisplayType {
	#[default]
	#[strum(serialize = "normal")]
	Normal, // normal ... お知らせページ掲載
	#[strum(serialize = "banner")]
	Banner, // banner ... お知らせページ掲載 + バナー表示
	#[strum(serialize = "dialog")]
	Dialog, // dialog ... お知らせページ掲載 + ダイアログ表示
}
