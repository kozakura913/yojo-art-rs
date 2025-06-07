use std::{
	collections::{HashMap, HashSet},
	sync::Arc,
};

use chrono::Utc;
use redis::{AsyncCommands, aio::MultiplexedConnection};
use serde::{Deserialize, Serialize};

use crate::{
	DataBase, MisskeyConfig, ParsedMisskeyConfig, ServerError,
	models::{
		following::MiFollowing,
		user::{MiAvatarDecoration, MiUser},
		user_memo::MiUserMemo,
		user_note_pining::MiUserNotePining,
		user_profile::MiUserProfile,
	},
};

use super::{
	announcement::AnnouncementService, emoji::EmojiService, id_service::IdService,
	instance::InstanceService, meta::MetaService, role::RoleService,
};

pub const USER_ONLINE_THRESHOLD: i64 = 1000 * 60 * 10; // 10min
pub const USER_ACTIVE_THRESHOLD: i64 = 1000 * 60 * 60 * 24 * 3; // 3days
#[derive(Clone, Debug)]
pub struct UserService {
	config: Arc<ParsedMisskeyConfig>,
	redis: MultiplexedConnection,
	db: DataBase,
	id_service: IdService,
	role_service: RoleService,
	announcement_service: AnnouncementService,
	emoji_service: EmojiService,
	instance_service: InstanceService,
	meta_service: MetaService,
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
		config: Arc<ParsedMisskeyConfig>,
		redis: MultiplexedConnection,
		db: DataBase,
		id_service: IdService,
		role_service: RoleService,
		announcement_service: AnnouncementService,
		emoji_service: EmojiService,
		instance_service: InstanceService,
		meta_service: MetaService,
	) -> Self {
		Self {
			config,
			redis,
			db,
			id_service,
			role_service,
			announcement_service,
			emoji_service,
			instance_service,
			meta_service,
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
		use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
		use diesel_async::RunQueryDsl;
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
						eprintln!("{}:{} {:?}", file!(), line!(), e);
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
					eprintln!("{}:{} {:?}", file!(), line!(), e);
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
					eprintln!("{}:{} {:?}", file!(), line!(), e);
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
					eprintln!("{}:{} {:?}", file!(), line!(), e);
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
					eprintln!("{}:{} {:?}", file!(), line!(), e);
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
					eprintln!("{}:{} {:?}", file!(), line!(), e);
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
					eprintln!("{}:{} {:?}", file!(), line!(), e);
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
					eprintln!("{}:{} {:?}", file!(), line!(), e);
				})
				.ok();
			res.is_some()
		};
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
			"{}identicon/{}@{}",
			self.config.url,
			user.username.to_lowercase(),
			user.host
				.as_ref()
				.map(|s| s.as_str())
				.unwrap_or(self.config.host.as_str())
		)
	}
	pub async fn pack_lite(&self, user: MiUser) -> Result<PackedUserLite, ServerError> {
		let online_status = self.online_status(&user);
		let avatar_url = if user.avatar_url.is_none() {
			self.identicon_url(&user)
		} else {
			user.avatar_url.unwrap()
		};
		//println!("avatar_decorations={:?}", user.avatar_decorations);
		let mut con = self.db.get().await.ok_or("db")?;
		let instance = match user.host.as_ref() {
			Some(host) => Some(
				self.instance_service
					.fetch_connection((&mut con).into(), host)
					.await?,
			),
			None => None,
		};
		let meta = self.meta_service.load(false).await.ok_or("meta")?;
		let avatar_decorations = user.avatar_decorations.into_inner();
		let mut avatar_decoration_ids = HashSet::new();
		for ad in avatar_decorations.iter() {
			avatar_decoration_ids.insert(ad.id.clone());
		}
		let avatar_decoration_ids: Vec<String> = avatar_decoration_ids.into_iter().collect();
		let avatar_decoration_urls = async {
			use crate::models::avatar_decoration::avatar_decoration::dsl::avatar_decoration;
			use crate::models::avatar_decoration::avatar_decoration::dsl::*;
			use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
			use diesel_async::RunQueryDsl;
			let res: Option<Vec<crate::models::avatar_decoration::MiAvatarDecoration>> =
				avatar_decoration
					.filter(id.eq_any(&avatar_decoration_ids))
					.select(crate::models::avatar_decoration::MiAvatarDecoration::as_select())
					.load(&mut con)
					.await
					.map_err(|e| {
						eprintln!("{}:{} {:?}", file!(), line!(), e);
					})
					.ok();
			res.map(|ad| {
				let mut map = HashMap::new();
				for ad in ad.into_iter() {
					map.insert(ad.id, ad.url);
				}
				map
			})
		};
		//DBクエリ
		let avatar_decoration_urls = avatar_decoration_urls.await;
		let avatar_decorations = avatar_decorations
			.into_iter()
			.map(|raw_avatar_decoration: MiAvatarDecoration| {
				let mut packed_avatar_decoration: PackedAvatarDecoration =
					raw_avatar_decoration.into();
				if let Some(Some(s)) = avatar_decoration_urls
					.as_ref()
					.map(|map| map.get(&packed_avatar_decoration.id))
				{
					packed_avatar_decoration.url.push_str(s.as_str());
				}
				packed_avatar_decoration
			})
			.collect();
		Ok(PackedUserLite {
			name: user.name,
			username: user.username,
			host: user.host.clone(),
			avatar_url,
			avatar_blurhash: user.avatar_blurhash,
			avatar_decorations,
			is_locked: user.is_locked,
			is_bot: user.is_bot,
			is_cat: user.is_cat,
			is_proxy: meta.other.proxy_account_id.as_ref() == Some(&user.id),
			require_signin_to_view_contents: user.require_signin_to_view_contents,
			make_notes_followers_only_before: user.make_notes_followers_only_before,
			make_notes_hidden_before: user.make_notes_hidden_before,
			instance: instance.map(|instance| PackedInstance {
				name: instance.name,
				softwareName: instance.softwareName,
				softwareVersion: instance.softwareVersion,
				iconUrl: instance.iconUrl,
				faviconUrl: instance.faviconUrl,
				themeColor: instance.themeColor,
			}),
			emojis: self
				.emoji_service
				.populate_emojis(&mut con, user.emojis, user.host)
				.await,
			online_status,
			set_federation_avatar_shape: user.set_federation_avatar_shape,
			is_square_avatars: user.is_square_avatars,
			id: user.id,
		})
	}
	pub fn online_status(&self, user: &MiUser) -> OnlineStatus {
		if user.hide_online_status {
			OnlineStatus::unknown
		} else if let Some(last_active_date) = user.last_active_date {
			let elapsed = (Utc::now() - last_active_date.and_utc()).num_milliseconds();
			if elapsed < USER_ONLINE_THRESHOLD {
				OnlineStatus::online
			} else if elapsed < USER_ACTIVE_THRESHOLD {
				OnlineStatus::active
			} else {
				OnlineStatus::offline
			}
		} else {
			OnlineStatus::unknown
		}
	}
}
pub trait PackedUser:
	Clone + std::fmt::Debug + serde::ser::Serialize + serde::de::Deserialize<'static>
{
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackedUserLite {
	pub id: String,
	pub name: Option<String>,
	pub username: String,
	pub host: Option<String>,
	#[serde(rename = "avatarUrl")]
	pub avatar_url: String,
	#[serde(rename = "avatarBlurhash")]
	pub avatar_blurhash: Option<String>,
	#[serde(rename = "avatarDecorations")]
	pub avatar_decorations: Vec<PackedAvatarDecoration>,
	#[serde(rename = "isLocked")]
	pub is_locked: bool,
	#[serde(rename = "isBot")]
	pub is_bot: bool,
	#[serde(rename = "isCat")]
	pub is_cat: bool,
	#[serde(rename = "isProxy")]
	pub is_proxy: bool,
	#[serde(rename = "requireSigninToViewContents")]
	pub require_signin_to_view_contents: bool,
	#[serde(rename = "makeNotesFollowersOnlyBefore")]
	pub make_notes_followers_only_before: Option<i32>,
	#[serde(rename = "makeNotesHiddenBefore")]
	pub make_notes_hidden_before: Option<i32>,
	pub instance: Option<PackedInstance>,
	pub emojis: HashMap<String, String>, //K=emoji:V=url
	pub online_status: OnlineStatus,
	//badgeRoles:Option<>,
	#[serde(rename = "setFederationAvatarShape")]
	pub set_federation_avatar_shape: Option<bool>,
	#[serde(rename = "isSquareAvatars")]
	pub is_square_avatars: Option<bool>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnlineStatus {
	#[serde(rename = "unknown")]
	unknown,
	#[serde(rename = "online")]
	online,
	#[serde(rename = "active")]
	active,
	#[serde(rename = "offline")]
	offline,
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
	#[serde(skip_serializing_if = "Option::is_none")]
	angle: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "offsetY")]
	offset_x: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "offsetX")]
	offset_y: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	scale: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	opacity: Option<f64>,
	#[serde(rename = "flipH")]
	flip_h: bool,
	url: String,
}
impl From<MiAvatarDecoration> for PackedAvatarDecoration {
	fn from(value: MiAvatarDecoration) -> Self {
		Self {
			id: value.id,
			angle: value.angle,
			offset_x: value.offset_x,
			offset_y: value.offset_y,
			scale: value.scale,
			opacity: value.opacity,
			flip_h: value.flip_h.unwrap_or(false),
			url: String::new(),
		}
	}
}
