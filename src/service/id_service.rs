mod aid;
mod aidx;
mod meid;
mod meidg;
mod object_id;
mod misskey_ulid;

use std::{fmt::Debug, sync::Arc};

use crate::MisskeyConfig;

#[derive(Debug,Clone)]
pub struct IdService(Arc<(String,Box<dyn IdServiceImpl>)>);
trait IdServiceImpl:Debug+Send+Sync{
	fn is_safe_t(&self,t:i64)->bool;
	fn gen(&self,time: i64)->String;
	fn parse(&self,id: &str)->Option<i64>;
}
impl IdService{
	pub fn new(config: &MisskeyConfig)->Self{
		let method=config.id.to_lowercase();
		let inner:Box<dyn IdServiceImpl>=match method.as_ref() {
			"aid"=> Box::new(aid::AidService::new()),
			"aidx"=> Box::new(aidx::AidxService::new()),
			"meid"=> Box::new(meid::MeidService::new()),
			"meidg"=> Box::new(meidg::MeidgService::new()),
			"ulid"=>  Box::new(misskey_ulid::UlidService::new()),
			"objectid"=> Box::new(object_id::ObjectIdService::new()),
			_=>unimplemented!("IdService"),
		};
		Self(Arc::new((method,inner)))
	}
	pub fn is_safe_t(&self,t: i64)-> bool {
		self.0.1.is_safe_t(t)
	}
	/**
	 * 時間を元にIDを生成します(省略時は現在日時)
	 * @param time 日時 1970-01-01T00:00:00Zからの経過ミリ秒
	 */
	pub fn gen(&self,time: Option<i64>)->String {
		let now=chrono::Utc::now().timestamp_millis();
		let t=if let Some(time)=time{
			if time>now{
				now
			}else{
				time
			}
		}else{
			now
		};
		self.0.1.gen(t)
	}
	pub fn parse(&self,id: &str)->Option<chrono::DateTime<chrono::Utc>> {
		let time=self.0.1.parse(id)?;
		chrono::DateTime::from_timestamp_millis(time)
	}
}
