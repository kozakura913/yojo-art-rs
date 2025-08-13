use std::sync::Arc;

use redis::{AsyncCommands, aio::ConnectionManager};
use serde::{Deserialize, Serialize};

use crate::{MisskeyConfig, ParsedMisskeyConfig};

pub enum StreamChannels<'a> {
	Main(&'a String),
	Drive(&'a String),
	Note(&'a String),
}
impl StreamChannels<'_> {
	fn channel_id(&self) -> String {
		match self {
			StreamChannels::Main(user_id) => format!("mainStream:{}", user_id.as_str()),
			StreamChannels::Drive(user_id) => format!("driveStream:{}", user_id.as_str()),
			StreamChannels::Note(note_id) => format!("noteStream:{}", note_id.as_str()),
		}
	}
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum MainEventType {
	#[serde(rename = "driveFileCreated")]
	DriveFileCreated,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum DriveEventType {
	#[serde(rename = "fileCreated")]
	FileCreated,
	#[serde(rename = "fileDeleted")]
	FileDeleted,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum NoteEventType {
	#[serde(rename = "reacted")]
	NoteUpdated,
}
#[derive(Clone)]
pub struct EventService {
	redis: ConnectionManager,
	config: Arc<ParsedMisskeyConfig>,
}
#[derive(Debug)]
pub enum EventError {
	Json(serde_json::Error),
	Redis(redis::RedisError),
}
impl From<serde_json::Error> for EventError {
	fn from(value: serde_json::Error) -> Self {
		Self::Json(value)
	}
}
impl From<redis::RedisError> for EventError {
	fn from(value: redis::RedisError) -> Self {
		Self::Redis(value)
	}
}
#[derive(Clone, Serialize, Deserialize, Debug)]
struct Event{
	channel:String,
	message:serde_json::Value,
}
impl EventService {
	pub fn new(redis: ConnectionManager, config: Arc<ParsedMisskeyConfig>) -> Self {
		Self { redis, config }
	}
	async fn publish(
		&self,
		channel: StreamChannels<'_>,
		event_type: Option<serde_json::Value>,
		value: Option<serde_json::Value>,
	) -> Result<(), EventError> {
		let message = match (event_type, value) {
			(None, None) => serde_json::Value::Null,
			(None, Some(body)) => body,
			(Some(key), body) => {
				let mut map = serde_json::Map::new();
				map.insert("type".to_string(), key);
				map.insert("body".to_string(), body.into());
				serde_json::Value::Object(map)
			}
		};
		let event=Event{
			channel:channel.channel_id(),
			message,
		};
		let res = serde_json::to_string(&event)?;
		let mut r = self.redis.clone();
		let host = &self.config.host;
		println!("publish event {} {}", host,res);
		Ok(r.publish::<&str, String, ()>(host, res).await?)
	}
	pub async fn publish_main_stream(
		&self,
		user_id: &String,
		event_type: Option<MainEventType>,
		value: Option<serde_json::Value>,
	) -> Result<(), EventError> {
		let event_type = match event_type {
			Some(event_type) => Some(serde_json::to_value(event_type)?),
			None => None,
		};
		self.publish(StreamChannels::Main(user_id), event_type, value)
			.await
	}
	pub async fn publish_note_stream(
		&self,
		note_id: &String,
		event_type: Option<NoteEventType>,
		value: serde_json::Value,
	) -> Result<(), EventError> {
		let event_type = match event_type {
			Some(event_type) => Some(serde_json::to_value(event_type)?),
			None => None,
		};
		#[derive(Clone, Serialize, Deserialize, Debug)]
		struct EventBody{
			id:String,
			body:serde_json::Value,
		}
		let event=EventBody{
			id:note_id.clone(),
			body:value,
		};
		self.publish(StreamChannels::Note(note_id), event_type, Some(serde_json::to_value(event)?))
			.await
	}
	pub async fn publish_drive_stream(
		&self,
		user_id: &String,
		event_type: Option<DriveEventType>,
		value: Option<serde_json::Value>,
	) -> Result<(), EventError> {
		let event_type = match event_type {
			Some(event_type) => Some(serde_json::to_value(event_type)?),
			None => None,
		};
		self.publish(StreamChannels::Drive(user_id), event_type, value)
			.await
	}
}
