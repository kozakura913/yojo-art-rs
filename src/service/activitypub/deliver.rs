use std::collections::HashSet;

use apalis::prelude::*;
use apalis::{
	layers::{ErrorHandlingLayer, WorkerBuilderExt},
	prelude::WorkerFactoryFn,
};
use apalis_redis::RedisStorage;
use axum::BoxError;
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};

use crate::DataBase;
use crate::models::following::MiFollowerInbox;
use crate::models::user::MiUserInbox;
use crate::service::activitypub::signature;
use crate::service::activitypub::signature::APSignatureService;

pub enum DeliverTarget {
	Direct(String), //user_id
	Follower,       //me_id follower
}
#[derive(Clone, Debug)]
pub struct APDeliverService {
	db: DataBase,
	storage: RedisStorage<DeliverJob>,
}
impl APDeliverService {
	pub async fn new(
		sign: APSignatureService,
		db: DataBase,
		redis_for_job_queue: ConnectionManager,
	) -> Self {
		let config = apalis_redis::Config::default().set_namespace("deliver");
		let storage: RedisStorage<DeliverJob> =
			RedisStorage::new_with_config(redis_for_job_queue, config);
		let worker = WorkerBuilder::new("deliver-worker")
			.layer(ErrorHandlingLayer::new())
			.enable_tracing()
			.rate_limit(5, std::time::Duration::from_secs(1))
			//.timeout(std::time::Duration::from_millis(500))
			.concurrency(2)
			.data(sign)
			.backend(storage.clone())
			.build_fn(deliver);
		tokio::runtime::Handle::current().spawn(async move {
			apalis::prelude::Monitor::new()
				.register(worker)
				.on_event(|e| {
					let worker_id = e.id();
					match e.inner() {
						apalis::prelude::Event::Start => {
							println!("Worker [{worker_id}] started");
						}
						apalis::prelude::Event::Error(e) => {
							eprintln!("Worker [{worker_id}] encountered an error: {e}");
						}
						apalis::prelude::Event::Exit => {
							println!("Worker [{worker_id}] exited");
						}
						_ => {}
					}
				})
				.shutdown_timeout(std::time::Duration::from_millis(5000))
				.run_with_signal(async {
					println!("Monitor started");
					tokio::signal::ctrl_c().await?;
					println!("Monitor starting shutdown");
					Ok(())
				})
				.await
				.unwrap();
		});
		Self { db, storage }
	}
	pub async fn post(
		&self,
		target: impl Iterator<Item = DeliverTarget>,
		me_id: String,
		content: serde_json::Value,
	) -> Result<(), BoxError> {
		let mut dbcon = self.db.get_read_only().await?;
		let mut inbox_urls = HashSet::new();
		for target in target {
			match target {
				DeliverTarget::Direct(user_id) => {
					let target_user = MiUserInbox::load_by_id(&mut dbcon, &user_id).await?;
					if let Some(inbox) = target_user.shared_inbox {
						inbox_urls.insert(inbox);
					} else if let Some(inbox) = target_user.inbox {
						inbox_urls.insert(inbox);
					}
				}
				DeliverTarget::Follower => {
					let res: Vec<MiFollowerInbox> = {
						use crate::models::following::following::dsl::following;
						use crate::models::following::following::dsl::*;
						use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
						use diesel_async::RunQueryDsl;
						following
							.filter(followerHost.is_not_null())
							.filter(followeeId.eq(&me_id))
							.select(MiFollowerInbox::as_select())
							.load(&mut dbcon)
							.await
							.map_err(|e| {
								eprintln!("{}:{} {:?}", file!(), line!(), e);
								e
							})
					}?;
					for t in res {
						if let Some(shared_inbox) = t.follower_shared_inbox {
							inbox_urls.insert(shared_inbox);
						} else if let Some(inbox) = t.follower_inbox {
							inbox_urls.insert(inbox);
						}
					}
				}
			}
		}
		self.post_to_inbox(inbox_urls.into_iter(), me_id, content)
			.await;
		Ok(())
	}
	async fn post_to_inbox<T>(
		&self,
		inbox: impl Iterator<Item = T>,
		me_id: String,
		content: serde_json::Value,
	) where
		T: Into<String>,
	{
		let json = match serde_json::to_string(&content) {
			Ok(json) => json,
			Err(e) => {
				eprintln!("{}:{} {:?}", file!(), line!(), e);
				return;
			}
		};
		let hash = signature::sha256_digest(&json.as_bytes());
		let mut storage = self.storage.clone();
		for s in inbox {
			let res = storage
				.push(DeliverJob {
					inbox: s.into(),
					me_id: me_id.clone(),
					json: json.clone(),
					hash: hash.clone(),
				})
				.await;
			if let Err(e) = res {
				eprintln!("{}:{} {:?}", file!(), line!(), e);
			}
		}
	}
}
#[derive(Debug, Serialize, Deserialize, Clone)]
struct DeliverJob {
	pub inbox: String,
	pub me_id: String,
	pub json: String,
	pub hash: String,
}
async fn deliver(
	job: DeliverJob,
	sign: Data<APSignatureService>,
) -> Result<(), apalis::prelude::Error> {
	sign.post(job.inbox, job.me_id, &job.hash, job.json.into_bytes())
		.await?;
	Ok(())
}
