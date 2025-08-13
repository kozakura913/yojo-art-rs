use apalis::prelude::*;
use apalis::{layers::{ErrorHandlingLayer, WorkerBuilderExt}, prelude::WorkerFactoryFn};
use apalis_redis::RedisStorage;
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};

use crate::service::activitypub::signature;
use crate::{service::activitypub::signature::APSignatureService};

#[derive(Clone, Debug)]
pub struct APDeliverService{
	storage:RedisStorage<DeliverJob>,
}
impl APDeliverService{
	pub async fn new(sign: APSignatureService,redis_for_job_queue: ConnectionManager)->Self{
		let config = apalis_redis::Config::default().set_namespace("deliver");
		let storage:RedisStorage<DeliverJob> = RedisStorage::new_with_config(redis_for_job_queue, config);
		let worker = WorkerBuilder::new("deliver-worker")
			.layer(ErrorHandlingLayer::new())
			.enable_tracing()
			.rate_limit(5, std::time::Duration::from_secs(1))
			//.timeout(std::time::Duration::from_millis(500))
			.concurrency(2)
			.data(sign)
			.backend(storage.clone())
			.build_fn(deliver);
		tokio::runtime::Handle::current().spawn(async move{
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
			.await.unwrap();
		});
		Self { storage}
	}
	pub async fn post<T>(&self,inbox:impl Iterator<Item=T>,me_id:String,content:serde_json::Value) where T:Into<String>{
		let json=match serde_json::to_string(&content){
			Ok(json)=>json,
			Err(e)=>{
				eprintln!("{}:{} {:?}",file!(),line!(),e);
				return;
			}
		};
		let hash=signature::sha256_digest(&json.as_bytes());
		let mut storage=self.storage.clone();
		for s in inbox{
			let res=storage.push(DeliverJob{
				inbox:s.into(),
				me_id:me_id.clone(),
				json:json.clone(),
				hash:hash.clone(),
			}).await;
			if let Err(e)=res{
				eprintln!("{}:{} {:?}",file!(),line!(),e);
			}
		}
	}
}
#[derive(Debug, Serialize, Deserialize, Clone)]
struct DeliverJob {
	pub inbox:String,
	pub me_id:String,
	pub json: String,
	pub hash: String,
}
async fn deliver(job:DeliverJob,sign: Data<APSignatureService>)->Result<(),apalis::prelude::Error>{
	sign.post(job.inbox, job.me_id,&job.hash,job.json.into_bytes()).await?;
	Ok(())
}
