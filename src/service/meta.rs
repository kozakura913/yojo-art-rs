use std::sync::Arc;

use diesel::{QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use tokio::sync::RwLock;

use crate::{
	DataBase,
	models::meta::{
		MiMeta, branding::MiMetaBranding, moderation::MiMetaModeration, other::MiMetaOther,
	},
};

#[derive(Clone, Debug)]
pub struct MetaService {
	db: DataBase,
	cache: Arc<RwLock<Option<Arc<MiMeta>>>>,
}
impl MetaService {
	pub fn new(db: DataBase) -> Self {
		Self {
			db,
			cache: Arc::new(RwLock::new(None)),
		}
	}
	/**
	 * キャッシュからしか取得しない。通常はload(true)を使うこと
	 */
	pub async fn fetch(&self) -> Option<Arc<MiMeta>> {
		let rl = self.cache.read().await;
		rl.clone()
	}
	/**
	 * allow_cache=false指定でキャッシュを更新する
	 */
	pub async fn load(&self, allow_cache: bool) -> Option<Arc<MiMeta>> {
		if allow_cache {
			if let Some(v) = self.fetch().await {
				return Some(v);
			}
		}
		let mut con = self.db.get().await?;
		let other: MiMetaOther = {
			use crate::models::meta::other::meta::dsl::meta;
			meta.select(MiMetaOther::as_select())
				.first(&mut con)
				.await
				.map_err(|e| {
					eprintln!("{:?}", e);
				})
		}
		.ok()?;
		let branding: MiMetaBranding = {
			use crate::models::meta::branding::meta::dsl::meta;
			meta.select(MiMetaBranding::as_select())
				.first(&mut con)
				.await
				.map_err(|e| {
					eprintln!("{:?}", e);
				})
		}
		.ok()?;
		let moderation: MiMetaModeration = {
			use crate::models::meta::moderation::meta::dsl::meta;
			meta.select(MiMetaModeration::as_select())
				.first(&mut con)
				.await
				.map_err(|e| {
					eprintln!("{:?}", e);
				})
		}
		.ok()?;
		let res = MiMeta {
			moderation,
			branding,
			other,
		};
		let v = Some(Arc::new(res));
		*self.cache.write().await = v.clone();
		v
	}
}
