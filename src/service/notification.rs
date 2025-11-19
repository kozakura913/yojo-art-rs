use std::{
	collections::{HashMap, HashSet},
	sync::{Arc, LazyLock},
};

use crate::{
	DataBase, ParsedMisskeyConfig, ServerError,
	service::{
		event::{EventService, MainEventType},
		id_service::IdService,
		note::{NoteService, PackedNote},
		user::{PackedUserLite, UserService},
	},
};
use redis::{AsyncCommands, aio::ConnectionManager};
use serde::Serialize;
use tokio::sync::RwLock;
use yojo_art_models::{note::MiNote, user::MiUser};

static NOTE_REQUIRED_NOTIFICATION_TYPES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
	let mut set = HashSet::new();
	for x in [
		"note",
		"mention",
		"reply",
		"renote",
		"renote:grouped",
		"quote",
		"reaction",
		"reaction:grouped",
		"pollEnded",
	] {
		set.insert(x);
	}
	set
});

pub trait MiNotification: Serialize {}

#[derive(Serialize)]
pub struct MiReactionNotification {
	#[serde(rename = "type")]
	notification_type: String,
	#[serde(rename = "noteId")]
	note_id: String,
	reaction: String,
}
impl MiNotification for MiReactionNotification {}
impl MiReactionNotification {
	pub fn new(note_id: String, reaction: String) -> Self {
		Self {
			notification_type: "reaction".into(),
			note_id,
			reaction,
		}
	}
}
#[derive(Clone)]
pub struct NotificationService {
	db: DataBase,
	redis: ConnectionManager,
	id_service: IdService,
	user_service: UserService,
	event_service: EventService,
	note_service: NoteService,
	misskey_config: Arc<ParsedMisskeyConfig>,
}
impl NotificationService {
	pub fn new(
		db: DataBase,
		redis: ConnectionManager,
		misskey_config: Arc<ParsedMisskeyConfig>,
		id_service: IdService,
		user_service: UserService,
		event_service: EventService,
		note_service: NoteService,
	) -> Self {
		Self {
			db,
			redis,
			misskey_config,
			id_service,
			event_service,
			user_service,
			note_service,
		}
	}
	pub async fn create_notification(
		&self,
		notifier_user: Option<String>,
		notifiee_user: &String,
		notification: impl MiNotification,
	) -> Result<serde_json::Value, ServerError> {
		let created_at = chrono::Utc::now();
		let mut redis_id: Option<String> = None;
		let mut notification = ServerError::map_err(
			serde_json::to_value(notification),
			"9218257c-bf47-46de-97ca-cc18d3c7056f",
		)?;
		let json = notification.as_object_mut().ok_or((
			"MiNotification type error",
			"7a2b6b9f-1834-4e32-b4be-1b92b2cdbc14",
		))?;
		let notification_id = self.id_service.gen_id(Some(created_at.timestamp()));
		json.insert(
			"id".into(),
			serde_json::Value::String(notification_id.clone()),
		);
		json.insert(
			"createdAt".into(),
			serde_json::Value::String(created_at.to_rfc3339()),
		);
		if let Some(notifier_user) = notifier_user.as_ref() {
			let notification_type = json.get("type").ok_or((
				"MiNotification type error",
				"9d7ca0fe-ddbc-4bb1-8481-4e5c54c4e553",
			))?;
			let notification_type = match notification_type {
				serde_json::Value::String(s) => s,
				_ => {
					return Err((
						"MiNotification type error",
						"9dbec0f3-1b9f-454c-aed8-2d86a6295e8b",
					)
						.into());
				}
			};
			if self
				.is_skip_notification(notifiee_user, notification_type, notifier_user)
				.await?
			{
				json.insert(
					"notifierId".into(),
					serde_json::Value::String(notifier_user.clone()),
				);
				return Ok(notification);
			}
		}
		if let Some(notifier_user) = notifier_user {
			json.insert(
				"notifierId".into(),
				serde_json::Value::String(notifier_user),
			);
		}
		let notification_str = notification.to_string();
		let mut redis_ref = self.redis.clone();
		/*
		let v: Result<Vec<(String, String)>, redis::RedisError>=redis_ref.xrange_all(format!("{}:notificationTimeline:{notifiee_user}",&self.misskey_config.host)).await;
		match v{
			Err(e)=>{
				eprintln!("{:?}",e);
			}
			Ok(v)=>{
				println!("{:?}",v);
			}
		}
		*/
		for _ in 0..5 {
			let res: Result<String, redis::RedisError> = redis_ref
				.xadd_maxlen(
					format!(
						"{}:notificationTimeline:{notifiee_user}",
						&self.misskey_config.host
					),
					redis::streams::StreamMaxlen::Approx(
						self.misskey_config.per_user_notifications_max_count as _,
					),
					self.to_xlist_id(&notification_id)?,
					&[("data", notification_str.as_str())],
				)
				.await;
			if let Err(e) = &res {
				eprintln!("{}:{} RedisError {:?}", file!(), line!(), e);
			}
			if let Ok(id) = res {
				redis_id = Some(id);
				break;
			}
		}
		let redis_id = ServerError::map_opt(
			redis_id,
			"notification redis",
			"ca3e7492-42c9-4d19-b6de-3928f9db5461",
		)?;
		let packed = self.pack(&notification, notifiee_user).await?;

		// Publish notification event
		let push = self
			.event_service
			.publish_main_stream(
				notifiee_user,
				Some(MainEventType::Notification),
				Some(packed.clone()),
			)
			.await;
		if let Err(e) = push {
			eprintln!("{}:{} {:?}", file!(), line!(), e);
		}
		let event_service = self.event_service.clone();
		let notifiee_user = notifiee_user.clone();
		let notification_type = notification
			.get("type")
			.unwrap()
			.as_str()
			.unwrap()
			.to_owned();
		// 2秒経っても(今回作成した)通知が既読にならなかったら「未読の通知がありますよ」イベントを発行する
		tokio::runtime::Handle::current().spawn(async move {
			if notification_type != "test" {
				// テスト通知の場合は即時発行
				tokio::time::sleep(tokio::time::Duration::from_secs(2000)).await;
			}
			let latest_read_notification_id: Result<String, redis::RedisError> = redis_ref
				.get(format!("latestReadNotification:${notifiee_user}"))
				.await;
			if let Ok(latest_read_notification_id) = latest_read_notification_id {
				if latest_read_notification_id >= redis_id {
					return;
				}
			}
			let push = event_service
				.publish_main_stream(
					&notifiee_user,
					Some(MainEventType::UnreadNotification),
					Some(packed),
				)
				.await;
			if let Err(e) = push {
				eprintln!("{}:{} {:?}", file!(), line!(), e);
			}
			//TODO プッシュ通知
		});
		Ok(notification)
	}
	async fn pack(
		&self,
		notification: &serde_json::Value,
		notifiee_user: &String,
	) -> Result<serde_json::Value, ServerError> {
		#[derive(Serialize)]
		struct PackedNotification {
			id: String,
			#[serde(rename = "createdAt")]
			created_at: String,
			#[serde(rename = "type")]
			notification_type: String,
			#[serde(skip_serializing_if = "Option::is_none")]
			#[serde(rename = "userId")]
			user_id: Option<String>,
			#[serde(skip_serializing_if = "Option::is_none")]
			reaction: Option<String>,
			#[serde(skip_serializing_if = "Option::is_none")]
			user: Option<PackedUserLite>,
			#[serde(skip_serializing_if = "Option::is_none")]
			note: Option<PackedNote>,
		}
		let created_at = ServerError::map_opt(
			notification.get("createdAt"),
			"notification id",
			"fc29e788-50d3-4952-ad6e-e05a4e6eccff",
		)?;
		let created_at = ServerError::map_opt(
			created_at.as_str(),
			"notification id",
			"1945c4f3-9242-4030-926a-07a055b07a46",
		)?;
		let id = ServerError::map_opt(
			notification.get("id"),
			"notification id",
			"fc29e788-50d3-4952-ad6e-e05a4e6eccff",
		)?;
		let id = ServerError::map_opt(
			id.as_str(),
			"notification id",
			"1945c4f3-9242-4030-926a-07a055b07a46",
		)?;
		let notification_type = ServerError::map_opt(
			notification.get("type"),
			"notification type",
			"fc29e788-50d3-4952-ad6e-e05a4e6eccff",
		)?;
		let notification_type = ServerError::map_opt(
			notification_type.as_str(),
			"notification type",
			"1945c4f3-9242-4030-926a-07a055b07a46",
		)?;
		let user = if let Some(user_id) = notification
			.get("notifierId")
			.map(|v| v.as_str())
			.unwrap_or_default()
		{
			let mut db = ServerError::map_err(
				self.db.get_read_only().await,
				"e7d0a16b-7a1b-4cd3-b655-c1329e147987",
			)?;
			let note = ServerError::map_err(
				MiUser::load_by_id(&mut db, user_id).await,
				"97b10cbe-7ee9-4036-ab51-1aed1b575135",
			)?;
			Some(self.user_service.pack_lite(note).await?)
		} else {
			None
		};
		let note = if let Some(note_id) = notification
			.get("noteId")
			.map(|v| v.as_str())
			.unwrap_or_default()
			&& NOTE_REQUIRED_NOTIFICATION_TYPES.contains(notification_type)
		{
			let mut db = ServerError::map_err(
				self.db.get_read_only().await,
				"ab5d53c0-bb7c-4aa6-9779-1ad392079fd7",
			)?;
			let note = ServerError::map_err(
				MiNote::load_by_id(&mut db, note_id).await,
				"22120e27-c33e-4b3e-874e-9da7d9039cd8",
			)?;
			let user_cache = Arc::new(RwLock::new(HashMap::new()));
			Some(
				self.note_service
					.pack(note, Some(notifiee_user), user_cache)
					.await?,
			)
		} else {
			None
		};
		ServerError::map_err(
			serde_json::to_value(&PackedNotification {
				id: id.to_owned(),
				created_at: created_at.to_owned(),
				notification_type: notification_type.to_owned(),
				user_id: notification
					.get("notifierId")
					.map(|v| v.as_str().map(|s| s.to_string()))
					.unwrap_or_default(),
				user,
				note,
				reaction: if notification_type == "reaction" {
					notification
						.get("reaction")
						.map(|v| v.as_str().map(|s| s.to_string()))
						.unwrap_or_default()
				} else {
					None
				},
			}),
			"efeaba74-2602-43ce-8fa3-306cba6014c9",
		)
	}
	async fn is_skip_notification(
		&self,
		notifiee: &str,
		notification_type: &String,
		notifier: &str,
	) -> Result<bool, ServerError> {
		//TODO ユーザーミュート
		let notification_config = self
			.user_service
			.notification_recieve_config(&notifiee)
			.await;
		let notification_config = match notification_config {
			Ok(v) => v,
			Err(e) => {
				return Err(ServerError::id(
					format!("{}:{} {:?}", file!(), line!(), e),
					uuid::uuid!("a1933741-55f0-4791-a962-2643071f886d"),
				));
			}
		};
		let config = notification_config.get_by_name(notification_type);
		let config = match config {
			Some(v) => v,
			None => return Ok(false),
		};
		Ok(match config {
			yojo_art_models::user_profile::NotificationRecieveType::All => false, //全購読
			yojo_art_models::user_profile::NotificationRecieveType::Never => true, //全スキップ
			yojo_art_models::user_profile::NotificationRecieveType::Following => {
				//notifieeのフォローを購読
				let followings = self.user_service.followings(notifiee).await;
				let followings =
					ServerError::map_err(followings, "d264e908-f7b4-4448-be5e-bbe139e6c16c")?;
				followings.contains(notifier)
			}
			yojo_art_models::user_profile::NotificationRecieveType::Follower => {
				//notifieeのフォロワーを購読
				let followings = self.user_service.followings(notifier).await;
				let followings =
					ServerError::map_err(followings, "77d1d333-1abf-45b8-b203-2a594d185873")?;
				followings.contains(notifiee)
			}
			yojo_art_models::user_profile::NotificationRecieveType::MutualFollow
			| yojo_art_models::user_profile::NotificationRecieveType::FollowingOrFollower => {
				let followings = self.user_service.followings(notifiee);
				let followers = self.user_service.followings(notifier);
				let (followings, followers) =
					futures_util::future::join(followings, followers).await;
				let followings =
					ServerError::map_err(followings, "55073448-9231-48fc-8757-f73da43ab647")?;
				let followers =
					ServerError::map_err(followers, "6573e495-d301-4639-bf73-3556765c14c3")?;
				if *config == yojo_art_models::user_profile::NotificationRecieveType::MutualFollow {
					//相互フォロー関係
					followings.contains(notifier) && followers.contains(notifiee)
				} else {
					//notifieeのフォローもしくはフォロワー
					followings.contains(notifier) || followers.contains(notifiee)
				}
			}
			yojo_art_models::user_profile::NotificationRecieveType::List(list_id) => {
				//指定したユーザーのリストに含まれる場合購読
				list_id.contains(notifier)
			}
		})
	}
	fn to_xlist_id(&self, id: &str) -> Result<String, ServerError> {
		let (date, additional) = ServerError::map_opt(
			self.id_service.parse_full(id),
			"parse id full",
			"ce8f5941-f66b-4b88-94b3-b17fa03d948d",
		)?;
		let id = date.timestamp_micros().to_string() + "-0"; // + &additional.to_string();
		println!("to_xlist_id:{}", id);
		Ok(id)
	}
}
