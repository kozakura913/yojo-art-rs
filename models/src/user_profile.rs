use diesel::pg::sql_types::Jsonb;
use diesel::{
	Selectable,
	deserialize::{FromSql, FromSqlRow},
	expression::AsExpression,
	serialize::ToSql,
	sql_types::VarChar,
};
use field_accessor::FieldAccessor;
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
		notificationRecieveConfig -> Jsonb,
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
	#[diesel(column_name = "notificationRecieveConfig")]
	pub notification_recieve_config:NotificationRecieveConfig,
}

#[derive(
	PartialEq, Eq, Clone, Default, Debug, Serialize, Deserialize, FromSqlRow, AsExpression, PgJson,FieldAccessor,
)]
#[diesel(sql_type = Jsonb)]
pub struct NotificationRecieveConfig{
	note:Option<NotificationRecieveType>,
	follow:Option<NotificationRecieveType>,
	mention:Option<NotificationRecieveType>,
	reply:Option<NotificationRecieveType>,
	renote:Option<NotificationRecieveType>,
	quote:Option<NotificationRecieveType>,
	reaction:Option<NotificationRecieveType>,
	#[serde(rename = "pollEnded")]
	poll_ended:Option<NotificationRecieveType>,
	#[serde(rename = "receiveFollowRequest")]
	receive_follow_request:Option<NotificationRecieveType>,
	#[serde(rename = "followRequestAccepted")]
	follow_request_accepted:Option<NotificationRecieveType>,
	#[serde(rename = "groupInvited")]
	group_invited:Option<NotificationRecieveType>,
	#[serde(rename = "roleAssigned")]
	role_assigned:Option<NotificationRecieveType>,
	#[serde(rename = "achievementEarned")]
	achievement_earned:Option<NotificationRecieveType>,
	#[serde(rename = "exportCompleted")]
	export_completed:Option<NotificationRecieveType>,
	login:Option<NotificationRecieveType>,
	#[serde(rename = "createToken")]
	create_token:Option<NotificationRecieveType>,
	#[serde(rename = "scheduleNote")]
	schedule_note:Option<NotificationRecieveType>,
	app:Option<NotificationRecieveType>,
	test:Option<NotificationRecieveType>,
}
impl NotificationRecieveConfig{
	pub fn get_by_name(&self,name:&String)->Option<&NotificationRecieveType>{
		self.get(name).ok().map(|t|t.as_ref()).unwrap_or_default()
	}
}
#[derive(
	PartialOrd,PartialEq, Eq, Clone, Debug, Serialize, Deserialize,
)]
pub enum NotificationRecieveType{
	#[serde(rename = "all")]
	All,
	#[serde(rename = "never")]
	Never,
	#[serde(rename = "following")]
	Following,
	#[serde(rename = "follower")]
	Follower,
	#[serde(rename = "mutualFollow")]
	MutualFollow,
	#[serde(rename = "followingOrFollower")]
	FollowingOrFollower,
	#[serde(rename = "list")]
	List(String),
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
	pub async fn load_by_user(
		con: &mut diesel_async::pooled_connection::bb8::PooledConnection<'_, diesel_async::AsyncPgConnection>,
		user_id: &str,
	) -> Result<Self, diesel::result::Error> {
		use self::user_profile::dsl::user_profile;
		use self::user_profile::dsl::*;
		use diesel::{SelectableHelper,ExpressionMethods, QueryDsl};
		use diesel_async::RunQueryDsl;
		user_profile.filter(userId.eq(user_id))
			.select(Self::as_select())
			.first(con)
			.await
			.map_err(|e| {
				eprintln!("{}:{} {:?}", file!(), line!(), e);
				e
			})
	}
}
