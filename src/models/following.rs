use diesel::Selectable;

diesel::table! {
	#[sql_name = "following"]
	following (id) {
		id -> VarChar,
		followeeId -> VarChar,
		followerId -> VarChar,
		isFollowerHibernated -> Bool,
		withReplies -> Bool,
		notify -> Nullable<VarChar>,
		followerHost -> Nullable<VarChar>,
		followerInbox -> Nullable<VarChar>,
		followerSharedInbox -> Nullable<VarChar>,
		followeeHost -> Nullable<VarChar>,
		followeeInbox -> Nullable<VarChar>,
		followeeSharedInbox -> Nullable<VarChar>,
	}
}
#[derive(PartialEq,Eq,Debug,Clone,diesel::Insertable,diesel::Queryable,Selectable,diesel::QueryableByName)]
#[diesel(table_name = following)]
pub struct MiFollowing{
	pub id:String,
	#[diesel(column_name = "followeeId")]
	pub followee_id:String,
	#[diesel(column_name = "followerId")]
	pub follower_id:String,
	#[diesel(column_name = "isFollowerHibernated")]
	pub is_follower_hibernated:bool,
	#[diesel(column_name = "withReplies")]
	pub with_replies:bool,
	pub notify:Option<String>,
	#[diesel(column_name = "followerHost")]
	pub follower_host:Option<String>,
	#[diesel(column_name = "followerInbox")]
	pub follower_inbox:Option<String>,
	#[diesel(column_name = "followerSharedInbox")]
	pub follower_shared_inbox:Option<String>,
	#[diesel(column_name = "followeeHost")]
	pub followee_host:Option<String>,
	#[diesel(column_name = "followeeInbox")]
	pub followee_inbox:Option<String>,
	#[diesel(column_name = "followeeSharedInbox")]
	pub followee_shared_inbox:Option<String>,
}
