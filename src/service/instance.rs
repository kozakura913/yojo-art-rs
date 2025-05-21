use std::borrow::Cow;

use redis::{AsyncCommands, aio::MultiplexedConnection};
use serde::{Deserialize, Serialize};

use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;

use crate::{DBConnection, DBConnectionRef, DataBase, ServerError, models::instance::MiInstance};

#[derive(Clone, Debug)]
pub struct InstanceService {
	db: DataBase,
	redis: MultiplexedConnection,
}
impl InstanceService {
	pub fn new(db: DataBase, redis: MultiplexedConnection) -> Self {
		Self { db, redis }
	}
	pub async fn fetch(&self, host: impl AsRef<str>) -> Result<MiInstance, ServerError> {
		let mut con = self.db.get().await.ok_or("db")?;
		self.fetch_connection((&mut con).into(), host).await
	}
	pub async fn fetch_connection(
		&self,
		con: DBConnectionRef<'_, '_>,
		host: impl AsRef<str>,
	) -> Result<MiInstance, ServerError> {
		let host_name = host.as_ref();
		let mut redis = self.redis.clone();
		let cache = redis
			.get::<&str, String>(host_name)
			.await
			.map(|v| serde_json::from_str::<MiInstance>(&v));
		if let Ok(Ok(cache)) = cache {
			return Ok(cache);
		}

		let res: MiInstance = {
			use crate::models::instance::instance::dsl::instance;
			use crate::models::instance::instance::dsl::*;
			let query = instance
				.filter(host.eq(host_name))
				.select(MiInstance::as_select());
			match con {
				DBConnectionRef::Borrowed(con) => query.first(con).await,
				DBConnectionRef::Mutex(m) => {
					let mut con = m.lock().await;
					query.first(&mut con).await
				}
			}
			.map_err(|e| {
				eprintln!("{}:{} {:?}", file!(), line!(), e);
			})
		}?;
		if let Ok(json) = serde_json::to_string(&res) {
			let redis_res = redis
				.set_ex::<&str, String, ()>(host_name, json, 60 * 30)
				.await;
			if let Err(err) = redis_res {
				eprintln!("{:?}", err);
			}
		}
		Ok(res)
	}
}
