use std::{collections::HashMap, fmt::Debug, sync::Arc};

use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use memory_cache::MemoryCache;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tract_data::itertools::Itertools;

use crate::{
	DataBase,
	models::role::{MiRole, MiRoleAssignment, Policy},
};

use super::meta::MetaService;

#[derive(Clone)]
pub struct RoleService {
	db: DataBase,
	meta_service: MetaService,
	user_role_cache: Arc<RwLock<MemoryCache<String, Arc<Vec<MiRole>>>>>,
}
impl Debug for RoleService {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("RoleService")
			.field("db", &self.db)
			.field("meta_service", &self.meta_service)
			.finish()
	}
}
impl RoleService {
	pub fn new(db: DataBase, meta_service: MetaService) -> Self {
		Self {
			db,
			meta_service,
			user_role_cache: Arc::new(RwLock::new(MemoryCache::new())),
		}
	}
	pub async fn is_moderator(&self, user_id: &str) -> bool {
		let roles = match self.get_user_roles(user_id).await {
			Some(v) => v,
			None => return false,
		};
		for role in roles.iter() {
			if role.is_moderator || role.is_administrator {
				return true;
			}
		}
		false
	}
	pub async fn is_administrator(&self, user_id: &str) -> bool {
		let roles = match self.get_user_roles(user_id).await {
			Some(v) => v,
			None => return false,
		};
		for role in roles.iter() {
			if role.is_administrator {
				return true;
			}
		}
		false
	}

	pub async fn get_user_assigns(&self, user_id: &str) -> Option<Vec<MiRoleAssignment>> {
		let mut con = self
			.db
			.get_read_only()
			.await
			.map_err(|e| eprintln!("{}:{} {:?}", file!(), line!(), e))
			.ok()?;
		let now = chrono::Utc::now().naive_utc().into();
		let assigns: Vec<MiRoleAssignment> = {
			use crate::models::role::role_assignment::dsl::role_assignment;
			use crate::models::role::role_assignment::*;
			use diesel_async::RunQueryDsl;
			role_assignment
				.filter(userId.eq(user_id))
				.select(MiRoleAssignment::as_select())
				.load(&mut con)
				.await
				.map_err(|e| {
					eprintln!("{}:{} {:?}", file!(), line!(), e);
				})
		}
		.ok()?;
		// 期限切れのロールを除外
		Some(
			assigns
				.into_iter()
				.filter(|a| {
					if let Some(expires_at) = a.expires_at {
						expires_at > now
					} else {
						true
					}
				})
				.collect(),
		)
	}
	pub async fn get_user_roles(&self, user_id: &str) -> Option<Arc<Vec<MiRole>>> {
		{
			let role_cache = self.user_role_cache.read().await;
			if let Some(v) = role_cache.get(&user_id.to_string()) {
				return Some(v.clone());
			}
		}
		let mut con = self
			.db
			.get_read_only()
			.await
			.map_err(|e| eprintln!("{}:{} {:?}", file!(), line!(), e))
			.ok()?;
		let roles: Vec<MiRole> = {
			use crate::models::role::role::dsl::role;
			use diesel_async::RunQueryDsl;
			role.select(MiRole::as_select())
				.load(&mut con)
				.await
				.map_err(|e| {
					eprintln!("{}:{} {:?}", file!(), line!(), e);
				})
		}
		.ok()?;
		let assigns = self.get_user_assigns(user_id).await;
		let assigns = assigns.map(|s| s.into_iter().map(|x| x.role_id).collect_vec());
		let assigned_roles = roles
			.into_iter()
			.filter(|r| {
				assigns
					.as_ref()
					.map(|assigns| assigns.contains(&r.id))
					.unwrap_or(false)
			})
			.collect_vec();
		let assigned_roles = Arc::new(assigned_roles);
		let mut role_cache = self.user_role_cache.write().await;
		role_cache.insert(
			user_id.to_string(),
			assigned_roles.clone(),
			Some(std::time::Duration::from_secs(6 * 60 * 60)),
		);
		Some(assigned_roles)
	}
	pub async fn get_user_policies(&self, user_id: Option<&str>) -> RolePolicies {
		let mut base_policies: HashMap<String, Policy> = DEFAULT_POLICIES.into();
		let meta = self.meta_service.load(true).await;
		if let Some(Some(map)) = meta.as_ref().map(|v| v.other.policies.as_object()) {
			for (k, v) in map {
				if let Some(default_value) = base_policies.get_mut(k) {
					*default_value = Policy {
						use_default: true,
						priority: -1,
						value: v.clone(),
					};
				}
			}
		}
		let user_id = match user_id {
			Some(user_id) => user_id,
			None => {
				let mut role_policies: HashMap<String, serde_json::Value> = HashMap::new();
				for (k, v) in base_policies {
					role_policies.insert(k, v.value);
				}
				return serde_json::from_value(serde_json::to_value(role_policies).unwrap())
					.unwrap();
			}
		};
		let mut policies = base_policies;
		let base_policies = policies.clone();
		let roles = self.get_user_roles(user_id).await;
		if let Some(roles) = roles.as_ref() {
			for role in roles.iter() {
				for (k, v) in &role.policies.0 {
					if let Some(base) = policies.get_mut(k) {
						if base.priority < v.priority {
							if v.use_default {
								base.value = base_policies.get(k).unwrap().value.clone();
							} else {
								base.value = v.value.clone();
							}
						}
					}
				}
			}
		}
		let mut role_policies: HashMap<String, serde_json::Value> = HashMap::new();
		for (k, v) in policies {
			role_policies.insert(k, v.value);
		}
		serde_json::from_value(serde_json::to_value(role_policies).unwrap()).unwrap()
	}
}

pub const DEFAULT_POLICIES: RolePolicies = RolePolicies {
	gtl_available: true,
	ltl_available: true,
	can_public_note: true,
	can_edit_note: true,
	mention_limit: 20,
	can_invite: false,
	invite_limit: 0,
	invite_limit_cycle: 60 * 24 * 7,
	invite_expiration_time: 0,
	can_manage_custom_emojis: false,
	can_manage_avatar_decorations: false,
	can_search_notes: false,
	can_advanced_search_notes: false,
	can_use_translator: true,
	can_hide_ads: false,
	drive_capacity_mb: 100,
	always_mark_nsfw: false,
	can_update_bio_media: true,
	pin_limit: 5,
	antenna_limit: 5,
	word_mute_limit: 200,
	webhook_limit: 3,
	clip_limit: 10,
	note_each_clips_limit: 200,
	user_list_limit: 10,
	user_each_user_lists_limit: 50,
	rate_limit_factor: 1.0,
	avatar_decoration_limit: 1,
	file_size_limit: 50,
	mutual_link_section_limit: 1,
	mutual_link_limit: 15,
};
#[derive(Clone, Serialize, Deserialize, Default, Debug)]
pub struct RolePolicies {
	#[serde(rename = "gtlAvailable")]
	pub gtl_available: bool,
	#[serde(rename = "ltlAvailable")]
	pub ltl_available: bool,
	#[serde(rename = "canPublicNote")]
	pub can_public_note: bool,
	#[serde(rename = "canEditNote")]
	pub can_edit_note: bool,
	#[serde(rename = "mentionLimit")]
	pub mention_limit: i32,
	#[serde(rename = "canInvite")]
	pub can_invite: bool,
	#[serde(rename = "inviteLimit")]
	pub invite_limit: i32,
	#[serde(rename = "inviteLimitCycle")]
	pub invite_limit_cycle: i64,
	#[serde(rename = "inviteExpirationTime")]
	pub invite_expiration_time: i64,
	#[serde(rename = "canManageCustomEmojis")]
	pub can_manage_custom_emojis: bool,
	#[serde(rename = "canManageAvatarDecorations")]
	pub can_manage_avatar_decorations: bool,
	#[serde(rename = "canSearchNotes")]
	pub can_search_notes: bool,
	#[serde(rename = "canAdvancedSearchNotes")]
	pub can_advanced_search_notes: bool,
	#[serde(rename = "canUseTranslator")]
	pub can_use_translator: bool,
	#[serde(rename = "canHideAds")]
	pub can_hide_ads: bool,
	#[serde(rename = "driveCapacityMb")]
	pub drive_capacity_mb: i64,
	#[serde(rename = "alwaysMarkNsfw")]
	pub always_mark_nsfw: bool,
	#[serde(rename = "canUpdateBioMedia")]
	pub can_update_bio_media: bool,
	#[serde(rename = "pinLimit")]
	pub pin_limit: i32,
	#[serde(rename = "antennaLimit")]
	pub antenna_limit: i32,
	#[serde(rename = "wordMuteLimit")]
	pub word_mute_limit: i64,
	#[serde(rename = "webhookLimit")]
	pub webhook_limit: i32,
	#[serde(rename = "clipLimit")]
	pub clip_limit: i64,
	#[serde(rename = "noteEachClipsLimit")]
	pub note_each_clips_limit: i64,
	#[serde(rename = "userListLimit")]
	pub user_list_limit: i64,
	#[serde(rename = "userEachUserListsLimit")]
	pub user_each_user_lists_limit: i64,
	#[serde(rename = "rateLimitFactor")]
	pub rate_limit_factor: f64,
	#[serde(rename = "avatarDecorationLimit")]
	pub avatar_decoration_limit: i32,
	#[serde(rename = "fileSizeLimit")]
	pub file_size_limit: i64,
	#[serde(rename = "mutualLinkSectionLimit")]
	pub mutual_link_section_limit: i32,
	#[serde(rename = "mutualLinkLimit")]
	pub mutual_link_limit: i32,
}
impl Into<HashMap<String, Policy>> for RolePolicies {
	fn into(self) -> HashMap<String, Policy> {
		let mut policies = HashMap::new();
		if let serde_json::Value::Object(map) = serde_json::to_value(self).unwrap() {
			for (k, v) in map {
				policies.insert(
					k,
					Policy {
						use_default: false,
						priority: -1,
						value: v,
					},
				);
			}
		}
		policies
	}
}
