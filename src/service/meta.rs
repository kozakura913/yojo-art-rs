
use std::sync::Arc;

use diesel::{QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use tokio::sync::RwLock;

use crate::{models::meta::MiMeta, DataBase};

#[derive(Clone,Debug)]
pub struct MetaService{
	db:DataBase,
	cache:Arc<RwLock<Option<Arc<MiMeta>>>>,
}
impl MetaService{
	pub fn new(db:DataBase)->Self{
		Self{
			db,
			cache:Arc::new(RwLock::new(None)),
		}
	}
	pub async fn fetch(&self)->Option<Arc<MiMeta>>{
		let rl=self.cache.read().await;
		rl.clone()
	}
	pub async fn load(&self,allow_cache :bool)->Option<Arc<MiMeta>>{
		if allow_cache{
			if let Some(v)=self.fetch().await{
				return Some(v);
			}
		}
		let mut con=self.db.get().await?;
		let res:MiMeta={
			use crate::models::meta::meta::dsl::meta;
			meta.select(MiMeta::as_select()).first(&mut con).await.map_err(|e|{
				eprintln!("{:?}",e);
			})
		}.ok()?;
		let v=Some(Arc::new(res));
		*self.cache.write().await=v.clone();
		v
	}
}
