use std::sync::Arc;

use redis::{aio::MultiplexedConnection, AsyncCommands};
use serde::{Deserialize, Serialize};

use crate::MisskeyConfig;

pub enum StreamChannels<'a>{
	Main(&'a String),
	Drive(&'a String),
}
impl StreamChannels<'_>{
	fn channel_id(&self)->String{
		match self{
			StreamChannels::Main(user_id) => format!("mainStream:{}",user_id.as_str()),
			StreamChannels::Drive(user_id) => format!("driveStream:{}",user_id.as_str()),
		}
	}
}
#[derive(Clone,Serialize,Deserialize,Debug)]
pub enum MainEventType{
	#[serde(rename = "driveFileCreated")]
	DriveFileCreated,
}
#[derive(Clone,Serialize,Deserialize,Debug)]
pub enum DriveEventType{
	#[serde(rename = "fileCreated")]
	FileCreated
}
#[derive(Clone,Debug)]
pub struct EventService{
	redis:MultiplexedConnection,
	config:Arc<MisskeyConfig>,
}
#[derive(Debug)]
pub enum EventError{
	Json(serde_json::Error),
	Redis(redis::RedisError),
	ConfigUrl(String),
}
impl From<serde_json::Error> for EventError{
	fn from(value: serde_json::Error) -> Self {
		Self::Json(value)
	}
}
impl From<redis::RedisError> for EventError{
	fn from(value: redis::RedisError) -> Self {
		Self::Redis(value)
	}
}
impl EventService{
	pub fn new(redis:MultiplexedConnection,config:Arc<MisskeyConfig>,)->Self{
		Self{
			redis,config
		}
	}
	async fn publish(&self,channel: StreamChannels<'_>, t: Option<serde_json::Value>, value: Option<serde_json::Value>)->Result<(),EventError>{
		let message=match (t,value){
			(None, None) => serde_json::Value::Null,
			(None, Some(body)) => body,
			(Some(key), body) => {
				let mut map=serde_json::Map::new();
				map.insert("type".to_string(),key);
				map.insert("body".to_string(),body.into());
				serde_json::Value::Object(map)
			},
		};
		let mut map=serde_json::Map::new();
		map.insert("channel".to_string(),channel.channel_id().into());
		map.insert("message".to_string(),message.into());
		let res=serde_json::to_string(&map)?;
		let mut r=self.redis.clone();
		let host=reqwest::Url::parse(&self.config.url).map_err(|e|EventError::ConfigUrl(e.to_string()))?;
		let host=host.host_str().ok_or_else(||EventError::ConfigUrl("NoHost".to_owned()))?;
		println!("publish event {}",host);
		Ok(r.publish::<&str,String,()>(host,res).await?)
	}
	pub async fn publish_main_stream(&self,user_id:&String,event_type: Option<MainEventType>, value: Option<serde_json::Value>)->Result<(),EventError>{
		let event_type=match event_type{
			Some(event_type)=>Some(serde_json::to_value(event_type)?),
			None=>None,
		};
		self.publish(StreamChannels::Main(user_id),event_type,value).await
	}
	pub async fn publish_drive_stream(&self,user_id:&String,event_type: Option<DriveEventType>, value: Option<serde_json::Value>)->Result<(),EventError>{
		let event_type=match event_type{
			Some(event_type)=>Some(serde_json::to_value(event_type)?),
			None=>None,
		};
		self.publish(StreamChannels::Drive(user_id),event_type,value).await
	}
}
