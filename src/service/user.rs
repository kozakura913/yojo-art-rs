use std::collections::HashMap;

use redis::{aio::MultiplexedConnection, AsyncCommands};

use crate::{models::{following::MiFollowing, user::MiUser, user_memo::MiUserMemo, user_note_pining::MiUserNotePining, user_profile::MiUserProfile}, DBConnection, DataBase};

use super::{announcement::AnnouncementService, id_service::IdService, role::RoleService};

#[derive(Clone,Debug)]
pub struct UserService{
	redis:MultiplexedConnection,
	db:DataBase,
	id_service:IdService,
	role_service:RoleService,
	announcement_service: AnnouncementService,
}
#[derive(Default,PartialEq,Eq,Debug)]
pub enum UserPackSchema{
	MeDetailed,
	UserDetailedNotMe,
	UserDetailed,
	#[default]
	UserLite,
}
#[derive(PartialEq,Eq,Clone,Debug)]
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
#[derive(Default,Clone,PartialEq, Eq,Debug)]
struct NotificationsInfo{
	hasUnread: bool,
	unreadCount: i32,
}
#[derive(Default,Debug)]
pub struct UserPackOptions{
	schema: UserPackSchema,
	includeSecrets: bool,
	userProfile: Option<MiUserProfile>,
	userRelations: Option<HashMap<String, UserRelation>>,
	userMemos: Option<HashMap<String,String>>,
	pinNotes:Option<HashMap<String,Vec<MiUserNotePining>>>,
}
impl UserService{
	pub fn new(
		redis:MultiplexedConnection,
		db:DataBase,
		id_service:IdService,
		role_service:RoleService,
		announcement_service:AnnouncementService,
	)->Self{
		Self{
			redis,
			db,
			id_service,
			role_service,
			announcement_service,
		}
	}
	pub async fn pack(&self,user:&MiUser,me_id:Option<&str>,opts:&UserPackOptions)->Option<serde_json::Value>{
		let is_detailed = opts.schema != UserPackSchema::UserLite;
		let is_me = me_id.map(|id|id==user.id).unwrap_or(false);
		let i_am_moderator = match me_id{
			Some(me_id)=>self.role_service.is_moderator(me_id).await,
			None=>false,
		};
		let mut con=self.db.get().await?;
		let profile = if is_detailed{
			MiUserProfile::load_by_user(&mut con,user.id.as_ref()).await
		}else{
			None
		};
		let mut relation =None;
		if me_id.is_some() && !is_me && is_detailed {
			if let Some(user_relations)=opts.userRelations.as_ref(){
				relation = user_relations.get(&user.id).cloned();
			} else {
				relation = self.get_relation(me_id.as_deref().unwrap(), user.id.as_str()).await;
			}
		}
		let mut memo = None;
		if is_detailed && me_id.is_some() {
			if let Some(memos)=opts.userMemos.as_ref(){
				memo = memos.get(&user.id).cloned();
			} else {
				memo = MiUserMemo::load_by_user(&mut con, me_id.as_ref().unwrap(),&user.id).await.map(|row|row.memo);
			}
		}

		let pins = if is_detailed{
			if let Some(pins)=opts.pinNotes.as_ref(){
				pins.get(&user.id).cloned().unwrap_or(vec![])
			} else {
				MiUserNotePining::load_by_user(&mut con,&user.id).await.unwrap_or(vec![])
			}
		}else{
			vec![]
		};
		let followingCount=if let Some(profile)=profile.as_ref(){
			if is_me || i_am_moderator || profile.following_visibility==crate::models::user_profile::Visibility::Public{
				Some(user.following_count)
			}else if profile.following_visibility==crate::models::user_profile::Visibility::Followers{
				let is_following=relation.as_ref().map(|r|r.is_following).unwrap_or(false);
				if is_following{
					Some(user.following_count)
				}else{
					None
				}
			}else{
				None
			}
		}else{
			None
		};
		let followersCount=if let Some(profile)=profile.as_ref(){
			if is_me || i_am_moderator || profile.followers_visibility==crate::models::user_profile::Visibility::Public{
				Some(user.followers_count)
			}else if profile.followers_visibility==crate::models::user_profile::Visibility::Followers{
				let is_following=relation.as_ref().map(|r|r.is_following).unwrap_or(false);
				if is_following{
					Some(user.followers_count)
				}else{
					None
				}
			}else{
				None
			}
		}else{
			None
		};
		let isModerator = if is_me && is_detailed {
			self.role_service.is_moderator(user.id.as_str()).await
		}else{
			false
		};
		let isAdmin = if is_me && is_detailed {
			self.role_service.is_administrator(user.id.as_str()).await
		}else{
			false
		};
		let unreadAnnouncements = if is_me && is_detailed{
			//createdAt: self.id_service.parse(announcement.id).date.toISOString(),
			self.announcement_service.get_unread_announcements(&user.id).await
		}else{
			None
		};
		let notificationsInfo = if is_me && is_detailed{
			self.getNotificationsInfo(&user.id).await
		}else{
			None
		};
		todo!("ユーザーのpackは未実装");
	}
	pub async fn get_relation(&self,me_id: &str, target: &str)-> Option<UserRelation>{
		let mut con=self.db.get().await?;
		let f_following=async move{
			let res:Option<MiFollowing>={
				use crate::models::following::following::dsl::following;
				use crate::models::following::following::dsl::*;
				following.filter(followerId.eq(me_id)).filter(followeeId.eq(target)).select(MiFollowing::as_select()).first(&mut con).await.map_err(|e|{
					eprintln!("{:?}",e);
				})
			}.ok();
			res
		};
		let mut con=self.db.get().await?;
		let f_is_followed=async move{
			use crate::models::following::following::dsl::following;
			use crate::models::following::following::dsl::*;
			let res:Option<crate::models::following::MiFollowing>=following.filter(followerId.eq(target)).filter(followeeId.eq(me_id)).first(&mut con).await.map_err(|e|{
				eprintln!("{:?}",e);
			}).ok();
			res.is_some()
		};
		let mut con=self.db.get().await?;
		let f_has_pending_follow_request_from_you=async move{
			use crate::models::follow_request::follow_request::dsl::follow_request;
			use crate::models::follow_request::follow_request::dsl::*;
			let res:Option<crate::models::follow_request::MiFollowRequest>=follow_request.filter(followerId.eq(me_id)).filter(followeeId.eq(target)).first(&mut con).await.map_err(|e|{
				eprintln!("{:?}",e);
			}).ok();
			res.is_some()
		};
		let mut con=self.db.get().await?;
		let f_has_pending_follow_request_to_you=async move{
			use crate::models::follow_request::follow_request::dsl::follow_request;
			use crate::models::follow_request::follow_request::dsl::*;
			let res:Option<crate::models::follow_request::MiFollowRequest>=follow_request.filter(followerId.eq(target)).filter(followeeId.eq(me_id)).first(&mut con).await.map_err(|e|{
				eprintln!("{:?}",e);
			}).ok();
			res.is_some()
		};
		let mut con=self.db.get().await?;
		let f_is_blocking=async move{
			use crate::models::blocking::blocking::dsl::blocking;
			use crate::models::blocking::blocking::dsl::*;
			let res:Option<crate::models::blocking::MiBlocking>=blocking.filter(blockerId.eq(me_id)).filter(blockeeId.eq(target)).first(&mut con).await.map_err(|e|{
				eprintln!("{:?}",e);
			}).ok();
			res.is_some()
		};
		let mut con=self.db.get().await?;
		let f_is_blocked=async move{
			use crate::models::blocking::blocking::dsl::blocking;
			use crate::models::blocking::blocking::dsl::*;
			let res:Option<crate::models::blocking::MiBlocking>=blocking.filter(blockerId.eq(target)).filter(blockeeId.eq(me_id)).first(&mut con).await.map_err(|e|{
				eprintln!("{:?}",e);
			}).ok();
			res.is_some()
		};
		let mut con=self.db.get().await?;
		let f_is_muted=async move{
			use crate::models::muting::muting::dsl::muting;
			use crate::models::muting::muting::dsl::*;
			let res:Option<crate::models::muting::MiMuting>=muting.filter(muterId.eq(me_id)).filter(muteeId.eq(target)).first(&mut con).await.map_err(|e|{
				eprintln!("{:?}",e);
			}).ok();
			res.is_some()
		};
		let mut con=self.db.get().await?;
		let f_is_renote_muted=async move{
			use crate::models::renote_muting::renote_muting::dsl::renote_muting;
			use crate::models::renote_muting::renote_muting::dsl::*;
			let res:Option<crate::models::renote_muting::MiRenoteMuting>=renote_muting.filter(muterId.eq(me_id)).filter(muteeId.eq(target)).first(&mut con).await.map_err(|e|{
				eprintln!("{:?}",e);
			}).ok();
			res.is_some()
		};
		use diesel::{QueryDsl, SelectableHelper,ExpressionMethods};
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
		)=futures_util::join!(
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
	async fn getNotificationsInfo(&self,userId: &str)->Option<NotificationsInfo>{
		let mut redis=self.redis.clone();

		let latestReadNotificationId = redis.get::<String,String>(format!("latestReadNotification:{}",userId)).await;

		let unreadCount=if let Ok(latestReadNotificationId)=latestReadNotificationId{
			let latestNotificationIdsRes = redis.xrevrange::<String,&str,String,Vec<String>>(
				format!("notificationTimeline:{}",userId),
				"+",
				latestReadNotificationId,
			).await.ok()?;
			if latestNotificationIdsRes.len() - 1 >= 0{
				latestNotificationIdsRes.len() as i32 - 1
			}else{
				0
			}
		} else {
			redis.xlen::<String,i32>(format!("notificationTimeline:{}",userId)).await.ok()?
		};
		Some(NotificationsInfo{
			unreadCount,
			hasUnread:unreadCount>0,
		})
	}
}
/*

	public async pack<S extends 'MeDetailed' | 'UserDetailedNotMe' | 'UserDetailed' | 'UserLite' = 'UserLite'>(
		src: MiUser['id'] | MiUser,
		me?: { id: MiUser['id']; } | null | undefined,
		options?: {
			schema?: S,
			includeSecrets?: boolean,
			userProfile?: MiUserProfile,
			userRelations?: Map<MiUser['id'], UserRelation>,
			userMemos?: Map<MiUser['id'], string | null>,
			pinNotes?: Map<MiUser['id'], MiUserNotePining[]>,
		},
	): Promise<Packed<S>> {
		const opts = Object.assign({
			schema: 'UserLite',
			includeSecrets: false,
		}, options);

		const user = typeof src === 'object' ? src : await this.usersRepository.findOneByOrFail({ id: src });

		const isDetailed = opts.schema !== 'UserLite';
		const meId = me ? me.id : null;
		const isMe = meId === user.id;
		const iAmModerator = me ? await this.roleService.isModerator(me as MiUser) : false;

		const profile = isDetailed
			? (opts.userProfile ?? await this.userProfilesRepository.findOneByOrFail({ userId: user.id }))
			: null;

		let relation: UserRelation | null = null;
		if (meId && !isMe && isDetailed) {
			if (opts.userRelations) {
				relation = opts.userRelations.get(user.id) ?? null;
			} else {
				relation = await this.getRelation(meId, user.id);
			}
		}

		let memo: string | null = null;
		if (isDetailed && meId) {
			if (opts.userMemos) {
				memo = opts.userMemos.get(user.id) ?? null;
			} else {
				memo = await this.userMemosRepository.findOneBy({ userId: meId, targetUserId: user.id })
					.then(row => row?.memo ?? null);
			}
		}

		let pins: MiUserNotePining[] = [];
		if (isDetailed) {
			if (opts.pinNotes) {
				pins = opts.pinNotes.get(user.id) ?? [];
			} else {
				pins = await this.userNotePiningsRepository.createQueryBuilder('pin')
					.where('pin.userId = :userId', { userId: user.id })
					.innerJoinAndSelect('pin.note', 'note')
					.orderBy('pin.id', 'DESC')
					.getMany();
			}
		}

		const followingCount = profile == null ? null :
			(profile.followingVisibility === 'public') || isMe || iAmModerator ? user.followingCount :
			(profile.followingVisibility === 'followers') && (relation && relation.isFollowing) ? user.followingCount :
			null;

		const followersCount = profile == null ? null :
			(profile.followersVisibility === 'public') || isMe || iAmModerator ? user.followersCount :
			(profile.followersVisibility === 'followers') && (relation && relation.isFollowing) ? user.followersCount :
			null;

		const isModerator = isMe && isDetailed ? this.roleService.isModerator(user) : null;
		const isAdmin = isMe && isDetailed ? this.roleService.isAdministrator(user) : null;
		const unreadAnnouncements = isMe && isDetailed ?
			(await this.announcementService.getUnreadAnnouncements(user)).map((announcement) => ({
				createdAt: this.idService.parse(announcement.id).date.toISOString(),
				...announcement,
			})) : null;

		const notificationsInfo = isMe && isDetailed ? await this.getNotificationsInfo(user.id) : null;
		//========WIP======

		const packed = {
			id: user.id,
			name: user.name,
			username: user.username,
			host: user.host,
			avatarUrl: user.avatarUrl ?? this.getIdenticonUrl(user),
			avatarBlurhash: user.avatarBlurhash,
			avatarDecorations: user.avatarDecorations.length > 0 ? this.avatarDecorationService.getAll(false, true).then(decorations => user.avatarDecorations.filter(ud => decorations.some(d => d.id === ud.id)).map(ud => ({
				id: ud.id,
				angle: ud.angle || undefined,
				flipH: ud.flipH || undefined,
				offsetX: ud.offsetX || undefined,
				offsetY: ud.offsetY || undefined,
				scale: ud.scale || undefined,
				opacity: ud.opacity || undefined,
				url: decorations.find(d => d.id === ud.id)!.url,
			}))) : [],
			isBot: user.isBot,
			isCat: user.isCat,
			instance: user.host ? this.federatedInstanceService.federatedInstanceCache.fetch(user.host).then(instance => instance ? {
				name: instance.name,
				softwareName: instance.softwareName,
				softwareVersion: instance.softwareVersion,
				iconUrl: instance.iconUrl,
				faviconUrl: instance.faviconUrl,
				themeColor: instance.themeColor,
			} : undefined) : undefined,
			emojis: this.customEmojiService.populateEmojis(user.emojis, user.host),
			onlineStatus: this.getOnlineStatus(user),
			// パフォーマンス上の理由でローカルユーザーのみ
			badgeRoles: user.host == null ? this.roleService.getUserBadgeRoles(user.id).then((rs) => rs
				.filter((r) => r.isPublic || iAmModerator)
				.sort((a, b) => b.displayOrder - a.displayOrder)
				.map((r) => ({
					name: r.name,
					iconUrl: r.iconUrl,
					displayOrder: r.displayOrder,
				}))
			) : undefined,

			...(isDetailed ? {
				url: profile!.url,
				uri: user.uri,
				movedTo: user.movedToUri ? this.apPersonService.resolvePerson(user.movedToUri).then(user => user.id).catch(() => null) : null,
				alsoKnownAs: user.alsoKnownAs
					? Promise.all(user.alsoKnownAs.map(uri => this.apPersonService.fetchPerson(uri).then(user => user?.id).catch(() => null)))
						.then(xs => xs.length === 0 ? null : xs.filter(x => x != null))
					: null,
				createdAt: this.idService.parse(user.id).date.toISOString(),
				updatedAt: user.updatedAt ? user.updatedAt.toISOString() : null,
				lastFetchedAt: user.lastFetchedAt ? user.lastFetchedAt.toISOString() : null,
				bannerUrl: user.bannerUrl,
				bannerBlurhash: user.bannerBlurhash,
				isLocked: user.isLocked,
				isSilenced: this.roleService.getUserPolicies(user.id).then(r => !r.canPublicNote),
				isSuspended: user.isSuspended,
				description: profile!.description,
				location: profile!.location,
				birthday: profile!.birthday,
				lang: profile!.lang,
				fields: profile!.fields,
				verifiedLinks: profile!.verifiedLinks,
				mutualLinkSections: profile!.mutualLinkSections,
				followersCount: followersCount ?? '?',
				followingCount: followingCount ?? '?',
				notesCount: user.notesCount,
				pinnedNoteIds: pins.map(pin => pin.noteId),
				pinnedNotes: this.noteEntityService.packMany(pins.map(pin => pin.note!), me, {
					detail: true,
				}),
				pinnedPageId: profile!.pinnedPageId,
				pinnedPage: profile!.pinnedPageId ? this.pageEntityService.pack(profile!.pinnedPageId, me) : null,
				publicReactions: this.isLocalUser(user) ? profile!.publicReactions : false, // https://github.com/misskey-dev/misskey/issues/12964
				followersVisibility: profile!.followersVisibility,
				followingVisibility: profile!.followingVisibility,
				twoFactorEnabled: profile!.twoFactorEnabled,
				usePasswordLessLogin: profile!.usePasswordLessLogin,
				securityKeys: profile!.twoFactorEnabled
					? this.userSecurityKeysRepository.countBy({ userId: user.id }).then(result => result >= 1)
					: false,
				roles: this.roleService.getUserRoles(user.id).then(roles => roles.filter(role => role.isPublic).sort((a, b) => b.displayOrder - a.displayOrder).map(role => ({
					id: role.id,
					name: role.name,
					color: role.color,
					iconUrl: role.iconUrl,
					description: role.description,
					isModerator: role.isModerator,
					isAdministrator: role.isAdministrator,
					displayOrder: role.displayOrder,
				}))),
				memo: memo,
				moderationNote: iAmModerator ? (profile!.moderationNote ?? '') : undefined,
			} : {}),

			...(isDetailed && isMe ? {
				avatarId: user.avatarId,
				bannerId: user.bannerId,
				isModerator: isModerator,
				isAdmin: isAdmin,
				injectFeaturedNote: profile!.injectFeaturedNote,
				receiveAnnouncementEmail: profile!.receiveAnnouncementEmail,
				alwaysMarkNsfw: profile!.alwaysMarkNsfw,
				autoSensitive: profile!.autoSensitive,
				carefulBot: profile!.carefulBot,
				autoAcceptFollowed: profile!.autoAcceptFollowed,
				noCrawle: profile!.noCrawle,
				preventAiLearning: profile!.preventAiLearning,
				isExplorable: user.isExplorable,
				isDeleted: user.isDeleted,
				twoFactorBackupCodesStock: profile?.twoFactorBackupSecret?.length === 5 ? 'full' : (profile?.twoFactorBackupSecret?.length ?? 0) > 0 ? 'partial' : 'none',
				hideOnlineStatus: user.hideOnlineStatus,
				hasUnreadSpecifiedNotes: this.noteUnreadsRepository.count({
					where: { userId: user.id, isSpecified: true },
					take: 1,
				}).then(count => count > 0),
				hasUnreadMentions: this.noteUnreadsRepository.count({
					where: { userId: user.id, isMentioned: true },
					take: 1,
				}).then(count => count > 0),
				hasUnreadAnnouncement: unreadAnnouncements!.length > 0,
				unreadAnnouncements,
				hasUnreadAntenna: this.getHasUnreadAntenna(user.id),
				hasUnreadChannel: false, // 後方互換性のため
				hasUnreadMessagingMessage: this.getHasUnreadMessagingMessage(user.id),
				hasUnreadNotification: notificationsInfo?.hasUnread, // 後方互換性のため
				hasPendingReceivedFollowRequest: this.getHasPendingReceivedFollowRequest(user.id),
				unreadNotificationsCount: notificationsInfo?.unreadCount,
				mutedWords: profile!.mutedWords,
				hardMutedWords: profile!.hardMutedWords,
				mutedInstances: profile!.mutedInstances,
				mutingNotificationTypes: [], // 後方互換性のため
				notificationRecieveConfig: profile!.notificationRecieveConfig,
				emailNotificationTypes: profile!.emailNotificationTypes,
				achievements: profile!.achievements,
				loggedInDays: profile!.loggedInDates.length,
				policies: this.roleService.getUserPolicies(user.id),
			} : {}),

			...(opts.includeSecrets ? {
				email: profile!.email,
				emailVerified: profile!.emailVerified,
				securityKeysList: profile!.twoFactorEnabled
					? this.userSecurityKeysRepository.find({
						where: {
							userId: user.id,
						},
						select: {
							id: true,
							name: true,
							lastUsed: true,
						},
					})
					: [],
			} : {}),

			...(relation ? {
				isFollowing: relation.isFollowing,
				isFollowed: relation.isFollowed,
				hasPendingFollowRequestFromYou: relation.hasPendingFollowRequestFromYou,
				hasPendingFollowRequestToYou: relation.hasPendingFollowRequestToYou,
				isBlocking: relation.isBlocking,
				isBlocked: relation.isBlocked,
				isMuted: relation.isMuted,
				isRenoteMuted: relation.isRenoteMuted,
				notify: relation.following?.notify ?? 'none',
				withReplies: relation.following?.withReplies ?? false,
			} : {}),
		} as Promiseable<Packed<S>>;

		return await awaitAll(packed);
	}

*/