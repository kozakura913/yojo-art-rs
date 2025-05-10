use std::{collections::HashMap, sync::Arc};

use redis::{AsyncCommands, aio::MultiplexedConnection};
use serde::{Deserialize, Serialize};

use crate::{
	DataBase, MisskeyConfig, ServerError,
	models::{
		following::MiFollowing, user::MiUser, user_memo::MiUserMemo,
		user_note_pining::MiUserNotePining, user_profile::MiUserProfile,
	},
};

use super::{announcement::AnnouncementService, id_service::IdService, role::RoleService};

#[derive(Clone, Debug)]
pub struct UserService {
	config: Arc<MisskeyConfig>,
	redis: MultiplexedConnection,
	db: DataBase,
	id_service: IdService,
	role_service: RoleService,
	announcement_service: AnnouncementService,
}
#[derive(Default, PartialEq, Eq, Debug)]
pub enum UserPackSchema {
	MeDetailed,
	UserDetailedNotMe,
	UserDetailed,
	#[default]
	UserLite,
}
#[derive(PartialEq, Eq, Clone, Debug)]
struct UserRelation {
	id: String,
	following: Option<MiFollowing>,
	is_following: bool,
	is_followed: bool,
	has_pending_follow_request_from_you: bool,
	has_pending_follow_request_to_you: bool,
	is_blocking: bool,
	is_blocked: bool,
	is_muted: bool,
	is_renote_muted: bool,
}
#[derive(Default, Clone, PartialEq, Eq, Debug)]
struct NotificationsInfo {
	hasUnread: bool,
	unreadCount: i32,
}
#[derive(Default, Debug)]
pub struct UserPackOptions {
	schema: UserPackSchema,
	includeSecrets: bool,
	userProfile: Option<MiUserProfile>,
	userRelations: Option<HashMap<String, UserRelation>>,
	userMemos: Option<HashMap<String, String>>,
	pinNotes: Option<HashMap<String, Vec<MiUserNotePining>>>,
}
impl UserService {
	pub fn new(
		config: Arc<MisskeyConfig>,
		redis: MultiplexedConnection,
		db: DataBase,
		id_service: IdService,
		role_service: RoleService,
		announcement_service: AnnouncementService,
	) -> Self {
		Self {
			config,
			redis,
			db,
			id_service,
			role_service,
			announcement_service,
		}
	}
	pub async fn pack(
		&self,
		user: &MiUser,
		me_id: Option<&str>,
		opts: &UserPackOptions,
	) -> Option<serde_json::Value> {
		let is_detailed = opts.schema != UserPackSchema::UserLite;
		let is_me = me_id.map(|id| id == user.id).unwrap_or(false);
		let i_am_moderator = match me_id {
			Some(me_id) => self.role_service.is_moderator(me_id).await,
			None => false,
		};
		let mut con = self.db.get().await?;
		let profile = if is_detailed {
			MiUserProfile::load_by_user(&mut con, user.id.as_ref()).await
		} else {
			None
		};
		let mut relation = None;
		if me_id.is_some() && !is_me && is_detailed {
			if let Some(user_relations) = opts.userRelations.as_ref() {
				relation = user_relations.get(&user.id).cloned();
			} else {
				relation = self
					.get_relation(me_id.as_deref().unwrap(), user.id.as_str())
					.await;
			}
		}
		let mut memo = None;
		if is_detailed && me_id.is_some() {
			if let Some(memos) = opts.userMemos.as_ref() {
				memo = memos.get(&user.id).cloned();
			} else {
				memo = MiUserMemo::load_by_user(&mut con, me_id.as_ref().unwrap(), &user.id)
					.await
					.map(|row| row.memo);
			}
		}

		let pins = if is_detailed {
			if let Some(pins) = opts.pinNotes.as_ref() {
				pins.get(&user.id).cloned().unwrap_or(vec![])
			} else {
				MiUserNotePining::load_by_user(&mut con, &user.id)
					.await
					.unwrap_or(vec![])
			}
		} else {
			vec![]
		};
		let followingCount = if let Some(profile) = profile.as_ref() {
			if is_me
				|| i_am_moderator
				|| profile.following_visibility == crate::models::user_profile::Visibility::Public
			{
				Some(user.following_count)
			} else if profile.following_visibility
				== crate::models::user_profile::Visibility::Followers
			{
				let is_following = relation.as_ref().map(|r| r.is_following).unwrap_or(false);
				if is_following {
					Some(user.following_count)
				} else {
					None
				}
			} else {
				None
			}
		} else {
			None
		};
		let followersCount = if let Some(profile) = profile.as_ref() {
			if is_me
				|| i_am_moderator
				|| profile.followers_visibility == crate::models::user_profile::Visibility::Public
			{
				Some(user.followers_count)
			} else if profile.followers_visibility
				== crate::models::user_profile::Visibility::Followers
			{
				let is_following = relation.as_ref().map(|r| r.is_following).unwrap_or(false);
				if is_following {
					Some(user.followers_count)
				} else {
					None
				}
			} else {
				None
			}
		} else {
			None
		};
		let isModerator = if is_me && is_detailed {
			self.role_service.is_moderator(user.id.as_str()).await
		} else {
			false
		};
		let isAdmin = if is_me && is_detailed {
			self.role_service.is_administrator(user.id.as_str()).await
		} else {
			false
		};
		let unreadAnnouncements = if is_me && is_detailed {
			//createdAt: self.id_service.parse(announcement.id).date.toISOString(),
			self.announcement_service
				.get_unread_announcements(&user.id)
				.await
		} else {
			None
		};
		let notificationsInfo = if is_me && is_detailed {
			self.getNotificationsInfo(&user.id).await
		} else {
			None
		};
		todo!("ユーザーのpackは未実装");
	}
	pub async fn get_relation(&self, me_id: &str, target: &str) -> Option<UserRelation> {
		let mut con = self.db.get().await?;
		let f_following = async move {
			let res: Option<MiFollowing> = {
				use crate::models::following::following::dsl::following;
				use crate::models::following::following::dsl::*;
				following
					.filter(followerId.eq(me_id))
					.filter(followeeId.eq(target))
					.select(MiFollowing::as_select())
					.first(&mut con)
					.await
					.map_err(|e| {
						eprintln!("{:?}", e);
					})
			}
			.ok();
			res
		};
		let mut con = self.db.get().await?;
		let f_is_followed = async move {
			use crate::models::following::following::dsl::following;
			use crate::models::following::following::dsl::*;
			let res: Option<crate::models::following::MiFollowing> = following
				.filter(followerId.eq(target))
				.filter(followeeId.eq(me_id))
				.first(&mut con)
				.await
				.map_err(|e| {
					eprintln!("{:?}", e);
				})
				.ok();
			res.is_some()
		};
		let mut con = self.db.get().await?;
		let f_has_pending_follow_request_from_you = async move {
			use crate::models::follow_request::follow_request::dsl::follow_request;
			use crate::models::follow_request::follow_request::dsl::*;
			let res: Option<crate::models::follow_request::MiFollowRequest> = follow_request
				.filter(followerId.eq(me_id))
				.filter(followeeId.eq(target))
				.first(&mut con)
				.await
				.map_err(|e| {
					eprintln!("{:?}", e);
				})
				.ok();
			res.is_some()
		};
		let mut con = self.db.get().await?;
		let f_has_pending_follow_request_to_you = async move {
			use crate::models::follow_request::follow_request::dsl::follow_request;
			use crate::models::follow_request::follow_request::dsl::*;
			let res: Option<crate::models::follow_request::MiFollowRequest> = follow_request
				.filter(followerId.eq(target))
				.filter(followeeId.eq(me_id))
				.first(&mut con)
				.await
				.map_err(|e| {
					eprintln!("{:?}", e);
				})
				.ok();
			res.is_some()
		};
		let mut con = self.db.get().await?;
		let f_is_blocking = async move {
			use crate::models::blocking::blocking::dsl::blocking;
			use crate::models::blocking::blocking::dsl::*;
			let res: Option<crate::models::blocking::MiBlocking> = blocking
				.filter(blockerId.eq(me_id))
				.filter(blockeeId.eq(target))
				.first(&mut con)
				.await
				.map_err(|e| {
					eprintln!("{:?}", e);
				})
				.ok();
			res.is_some()
		};
		let mut con = self.db.get().await?;
		let f_is_blocked = async move {
			use crate::models::blocking::blocking::dsl::blocking;
			use crate::models::blocking::blocking::dsl::*;
			let res: Option<crate::models::blocking::MiBlocking> = blocking
				.filter(blockerId.eq(target))
				.filter(blockeeId.eq(me_id))
				.first(&mut con)
				.await
				.map_err(|e| {
					eprintln!("{:?}", e);
				})
				.ok();
			res.is_some()
		};
		let mut con = self.db.get().await?;
		let f_is_muted = async move {
			use crate::models::muting::muting::dsl::muting;
			use crate::models::muting::muting::dsl::*;
			let res: Option<crate::models::muting::MiMuting> = muting
				.filter(muterId.eq(me_id))
				.filter(muteeId.eq(target))
				.first(&mut con)
				.await
				.map_err(|e| {
					eprintln!("{:?}", e);
				})
				.ok();
			res.is_some()
		};
		let mut con = self.db.get().await?;
		let f_is_renote_muted = async move {
			use crate::models::renote_muting::renote_muting::dsl::renote_muting;
			use crate::models::renote_muting::renote_muting::dsl::*;
			let res: Option<crate::models::renote_muting::MiRenoteMuting> = renote_muting
				.filter(muterId.eq(me_id))
				.filter(muteeId.eq(target))
				.first(&mut con)
				.await
				.map_err(|e| {
					eprintln!("{:?}", e);
				})
				.ok();
			res.is_some()
		};
		use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
		use diesel_async::RunQueryDsl;
		let (
			following,
			is_followed,
			has_pending_follow_request_from_you,
			has_pending_follow_request_to_you,
			is_blocking,
			is_blocked,
			is_muted,
			is_renote_muted,
		) = futures_util::join!(
			f_following,
			f_is_followed,
			f_has_pending_follow_request_from_you,
			f_has_pending_follow_request_to_you,
			f_is_blocking,
			f_is_blocked,
			f_is_muted,
			f_is_renote_muted,
		);

		Some(UserRelation {
			id: target.to_owned(),
			is_following: following.is_some(),
			following,
			is_followed,
			has_pending_follow_request_from_you,
			has_pending_follow_request_to_you,
			is_blocking,
			is_blocked,
			is_muted,
			is_renote_muted,
		})
	}
	async fn getNotificationsInfo(&self, userId: &str) -> Option<NotificationsInfo> {
		let mut redis = self.redis.clone();

		let latestReadNotificationId = redis
			.get::<String, String>(format!("latestReadNotification:{}", userId))
			.await;

		let unreadCount = if let Ok(latestReadNotificationId) = latestReadNotificationId {
			let latestNotificationIdsRes = redis
				.xrevrange::<String, &str, String, Vec<String>>(
					format!("notificationTimeline:{}", userId),
					"+",
					latestReadNotificationId,
				)
				.await
				.ok()?;
			if latestNotificationIdsRes.len() - 1 >= 0 {
				latestNotificationIdsRes.len() as i32 - 1
			} else {
				0
			}
		} else {
			redis
				.xlen::<String, i32>(format!("notificationTimeline:{}", userId))
				.await
				.ok()?
		};
		Some(NotificationsInfo {
			unreadCount,
			hasUnread: unreadCount > 0,
		})
	}
	pub fn is_remote_user(&self, user: &MiUser) -> bool {
		user.host.is_some()
	}
	pub fn identicon_url(&self, user: &MiUser) -> String {
		format!(
			"{}/identicon/{}@{}",
			self.config.url,
			user.username.to_lowercase(),
			user.host.as_ref().map(|s| s.as_str()).unwrap_or(".")
		)
	}
	pub async fn pack_lite(&self, user: MiUser) -> Result<PackedUserLite, ServerError> {
		let avatar_url = if user.avatar_url.is_none() {
			self.identicon_url(&user)
		} else {
			user.avatar_url.unwrap()
		};
		Ok(PackedUserLite {
			id: user.id,
			name: user.name,
			username: user.username,
			host: user.host,
			avatarUrl: avatar_url,
			avatarBlurhash: todo!(),
			avatarDecorations: todo!(),
			isLocked: todo!(),
			isBot: todo!(),
			isCat: todo!(),
			isProxy: todo!(),
			requireSigninToViewContents: todo!(),
			makeNotesFollowersOnlyBefore: todo!(),
			makeNotesHiddenBefore: todo!(),
			instance: todo!(),
			emojis: todo!(),
			onlineStatus: todo!(),
			setFederationAvatarShape: todo!(),
			isSquareAvatars: todo!(),
		})
	}
}
pub trait PackedUser: serde::ser::Serialize + serde::de::Deserialize<'static> {}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackedUserLite {
	id: String,
	name: Option<String>,
	username: String,
	host: Option<String>,
	avatarUrl: String,
	avatarBlurhash: Option<String>,
	avatarDecorations: Vec<PackedAvatarDecoration>,
	isLocked: bool,
	isBot: bool,
	isCat: bool,
	isProxy: bool,
	requireSigninToViewContents: bool,
	makeNotesFollowersOnlyBefore: i64,
	makeNotesHiddenBefore: i64,
	instance: Option<PackedInstance>,
	emojis: HashMap<String, String>, //K=emoji:V=url
	onlineStatus: String,            // "unknown" | "online" | "active" | "offline"
	//badgeRoles:Option<>,
	setFederationAvatarShape: bool,
	isSquareAvatars: bool,
}
impl PackedUser for PackedUserLite {}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackedInstance {
	name: Option<String>,
	softwareName: Option<String>,
	softwareVersion: Option<String>,
	iconUrl: Option<String>,
	faviconUrl: Option<String>,
	themeColor: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackedAvatarDecoration {
	id: String,
	angle: f64,
	offsetX: f64,
	offsetY: f64,
	scale: f64,
	opacity: f64,
	flipH: bool,
	url: String,
}
