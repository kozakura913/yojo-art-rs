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
use std::collections::HashMap;
use strum_macros::{Display, EnumString};

use crate::{DBConnection, models::common::UserSearchableBy};

//TODO テーブル分割したい(32制限に収めたい)
diesel::table! {
	#[sql_name = "user"]
	user (id) {
		id -> VarChar,
		updatedAt -> Nullable<Timestamp>,
		lastFetchedAt -> Nullable<Timestamp>,
		lastActiveDate -> Nullable<Timestamp>,
		hideOnlineStatus -> Bool,
		username -> VarChar,
		usernameLower -> VarChar,
		name -> Nullable<VarChar>,
		followersCount -> Int4,
		followingCount -> Int4,
		notesCount -> Int4,
		token -> Nullable<VarChar>,
		isDeleted -> Bool,
		emojis -> Array<VarChar>,
		host -> Nullable<VarChar>,
		avatarUrl -> Nullable<VarChar>,
		avatarBlurhash -> Nullable<VarChar>,
		avatarDecorations -> Jsonb,
		avatarId -> Nullable<VarChar>,
		bannerId -> Nullable<VarChar>,
		isSuspended -> Bool,
		isLocked -> Bool,
		isBot -> Bool,
		isCat -> Bool,
		isRoot -> Bool,
		isExplorable -> Bool,
		isIndexable -> Bool,
		searchableBy -> Nullable<crate::models::common::UserSearchableType>,
		requireSigninToViewContents -> Bool,
		makeNotesFollowersOnlyBefore -> Nullable<Int4>,
		makeNotesHiddenBefore -> Nullable<Int4>,
		setFederationAvatarShape -> Nullable<Bool>,
		isSquareAvatars -> Nullable<Bool>,
	}
}
#[derive(
	PartialEq,
	Debug,
	Clone,
	diesel::Insertable,
	diesel::Queryable,
	Selectable,
	diesel::QueryableByName,
	Serialize,
	Deserialize,
)]
#[diesel(table_name = user)]
pub struct MiUser {
	pub id: String,
	#[diesel(column_name = "updatedAt")]
	pub updated_at: Option<NaiveDateTime>,
	#[diesel(column_name = "lastFetchedAt")]
	pub last_fetched_at: Option<NaiveDateTime>,
	#[diesel(column_name = "lastActiveDate")]
	pub last_active_date: Option<NaiveDateTime>,
	#[diesel(column_name = "hideOnlineStatus")]
	pub hide_online_status: bool,
	pub name: Option<String>,
	pub username: String,
	#[diesel(column_name = "usernameLower")]
	pub username_lower: String,
	#[diesel(column_name = "name")]
	pub display_name: Option<String>,
	#[diesel(column_name = "followersCount")]
	pub followers_count: i32,
	#[diesel(column_name = "followingCount")]
	pub following_count: i32,
	#[diesel(column_name = "notesCount")]
	pub notes_count: i32,
	pub token: Option<String>, //リモートユーザーは持たない
	#[diesel(column_name = "isDeleted")]
	pub is_deleted: bool,
	pub emojis: Vec<String>,
	pub host: Option<String>, //ローカルユーザーは持たない
	#[diesel(column_name = "avatarUrl")]
	pub avatar_url: Option<String>,
	#[diesel(column_name = "avatarBlurhash")]
	pub avatar_blurhash: Option<String>,
	#[diesel(column_name = "avatarDecorations")]
	pub avatar_decorations: MiAvatarDecorations,
	#[diesel(column_name = "avatarId")]
	pub avatar_id: Option<String>,
	#[diesel(column_name = "bannerId")]
	pub banner_id: Option<String>,
	#[diesel(column_name = "isSuspended")]
	pub is_suspended: bool,
	#[diesel(column_name = "isLocked")]
	pub is_locked: bool,
	#[diesel(column_name = "isBot")]
	pub is_bot: bool,
	#[diesel(column_name = "isCat")]
	pub is_cat: bool,
	#[diesel(column_name = "isRoot")]
	pub is_root: bool,
	#[diesel(column_name = "isExplorable")]
	pub is_explorable: bool,
	#[diesel(column_name = "isIndexable")]
	pub is_indexable: bool,
	#[diesel(column_name = "searchableBy")]
	/** NoneでisIndexableを見る */
	pub searchable_by: Option<UserSearchableBy>,
	#[diesel(column_name = "makeNotesFollowersOnlyBefore")]
	/** in sec, マイナスで相対時間*/
	pub make_notes_followers_only_before: Option<i32>,
	#[diesel(column_name = "makeNotesHiddenBefore")]
	/** in sec, マイナスで相対時間*/
	pub make_notes_hidden_before: Option<i32>,
	#[diesel(column_name = "requireSigninToViewContents")]
	pub require_signin_to_view_contents: bool,
	#[diesel(column_name = "setFederationAvatarShape")]
	pub set_federation_avatar_shape: Option<bool>,
	#[diesel(column_name = "isSquareAvatars")]
	pub is_square_avatars: Option<bool>,
}
impl MiUser {
	pub async fn load_by_id(
		con: &mut DBConnection<'_>,
		user_id: &str,
	) -> Result<Self, diesel::result::Error> {
		use self::user::dsl::user;
		use self::user::dsl::*;
		user.filter(id.eq(user_id))
			.select(Self::as_select())
			.first(con)
			.await
	}
	pub async fn load_by_ids(
		con: &mut DBConnection<'_>,
		user_id: &Vec<String>,
	) -> Result<Vec<Self>, diesel::result::Error> {
		use self::user::dsl::user;
		use self::user::dsl::*;
		user.filter(id.eq_any(user_id))
			.select(Self::as_select())
			.load(con)
			.await
	}
	pub async fn load_by_token(
		con: &mut DBConnection<'_>,
		user_token: &str,
	) -> Result<Self, diesel::result::Error> {
		use self::user::dsl::user;
		use self::user::dsl::*;
		user.filter(token.eq(user_token))
			.select(Self::as_select())
			.first(con)
			.await
	}
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Jsonb)]
pub struct MiAvatarDecorations(Vec<MiAvatarDecoration>);
impl Into<Vec<MiAvatarDecoration>> for MiAvatarDecorations {
	fn into(self) -> Vec<MiAvatarDecoration> {
		self.0
	}
}
impl MiAvatarDecorations {
	pub fn into_inner(self) -> Vec<MiAvatarDecoration> {
		self.0
	}
}
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct MiAvatarDecoration {
	pub id: String,
	pub angle: Option<f64>,
	#[serde(rename = "flipH")]
	pub flip_h: Option<bool>,
	#[serde(rename = "offsetX")]
	pub offset_x: Option<f64>,
	#[serde(rename = "offsetY")]
	pub offset_y: Option<f64>,
	pub scale: Option<f64>,
	pub opacity: Option<f64>,
}
impl ToSql<Jsonb, diesel::pg::Pg> for MiAvatarDecorations
where
	serde_json::Value: ToSql<Jsonb, diesel::pg::Pg>,
{
	fn to_sql<'b>(
		&'b self,
		out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
	) -> diesel::serialize::Result {
		<serde_json::Value as ToSql<Jsonb, diesel::pg::Pg>>::to_sql(
			&(serde_json::to_value(&self).map_err(|e| Box::new(e))?),
			&mut out.reborrow(),
		)
	}
}
impl<DB: diesel::backend::Backend> FromSql<Jsonb, DB> for MiAvatarDecorations
where
	serde_json::Value: FromSql<Jsonb, DB>,
{
	fn from_sql(bytes: DB::RawValue<'_>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let v = <serde_json::Value as FromSql<Jsonb, DB>>::from_sql(bytes)?;
		Ok(serde_json::from_str::<MiAvatarDecorations>(&v.to_string()).map_err(|e| Box::new(e))?)
	}
}
