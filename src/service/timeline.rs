use crate::{
	DataBase, ServerError,
	models::{
		note::{MiNote, NoteVisibility},
		user::MiUser,
		user_profile::MiUserProfile,
	},
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct TimelineService {
	db: DataBase,
	//TODO キャッシュ
}
#[derive(Clone, Debug)]
pub struct TLOptions {
	pub until_id: Option<String>,
	pub since_id: Option<String>,
	pub with_files: bool,
	pub with_renotes: bool,
	pub allow_partial: bool,
	pub with_cats: bool,
	pub with_replies: bool,
	pub limit: u16,
}
#[derive(Default)]
pub struct TimelineHints {
	//返信、引用、リノート
	pub note_relation_note: HashMap<String, MiNote>,
	pub followings_user: Option<HashSet<String>>,
	pub muted_instances: Option<HashSet<String>>,
	pub renote_muting_user: Option<HashSet<String>>,
	pub is_muting_user: HashMap<String, bool>,
	pub user_cache: HashMap<String, MiUser>,
}
impl TimelineService {
	pub fn new(db: DataBase) -> Self {
		Self { db }
	}
	pub async fn get_stl(
		&self,
		me_id: &String,
		hints: &mut TimelineHints,
		opts: &TLOptions,
	) -> Result<Vec<MiNote>, ServerError> {
		let mut notes: Vec<MiNote> = Vec::new();
		let mut exclude_users = HashSet::new();
		for _ in 0..100 {
			let start = chrono::Utc::now();
			let mut append_notes: Vec<MiNote> = match self.raw_stl(me_id, &opts, hints).await {
				Ok(v) => v,
				Err(e) => {
					if notes.is_empty() {
						return Err(e.into());
					} else {
						return Ok(notes);
					}
				}
			};
			println!(
				"raw STL {}ms",
				(chrono::Utc::now() - start).num_milliseconds()
			);
			if append_notes.is_empty() {
				return Ok(notes);
			}
			let start = chrono::Utc::now();
			let _ = self
				.filter_note(
					&mut append_notes,
					Some(me_id),
					&mut exclude_users,
					hints,
					opts.with_cats,
				)
				.await?;
			println!(
				"filter_note {}ms",
				(chrono::Utc::now() - start).num_milliseconds()
			);
			if !append_notes.is_empty() {
				notes.extend_from_slice(&append_notes);
			}
			if !notes.is_empty() && (opts.allow_partial || notes.len() > opts.limit.into()) {
				return Ok(notes);
			}
		}
		Ok(notes)
	}
	pub async fn get_ltl(
		&self,
		me_id: Option<&String>,
		hints: &mut TimelineHints,
		opts: &TLOptions,
	) -> Result<Vec<MiNote>, ServerError> {
		let mut notes: Vec<MiNote> = Vec::new();
		let mut exclude_users = HashSet::new();
		for _ in 0..100 {
			let start = chrono::Utc::now();
			let mut append_notes: Vec<MiNote> = match self.raw_ltl(me_id, &opts, hints).await {
				Ok(v) => v,
				Err(e) => {
					if notes.is_empty() {
						return Err(e.into());
					} else {
						return Ok(notes);
					}
				}
			};
			println!(
				"raw LTL {}ms",
				(chrono::Utc::now() - start).num_milliseconds()
			);
			if append_notes.is_empty() {
				return Ok(notes);
			}
			let start = chrono::Utc::now();
			let _ = self
				.filter_note(
					&mut append_notes,
					me_id,
					&mut exclude_users,
					hints,
					opts.with_cats,
				)
				.await?;
			println!(
				"filter_note {}ms",
				(chrono::Utc::now() - start).num_milliseconds()
			);
			if !append_notes.is_empty() {
				notes.extend_from_slice(&append_notes);
			}
			if !notes.is_empty() && (opts.allow_partial || notes.len() > opts.limit.into()) {
				return Ok(notes);
			}
		}
		Ok(notes)
	}
	pub async fn get_htl(
		&self,
		me_id: &String,
		hints: &mut TimelineHints,
		opts: &TLOptions,
	) -> Result<Vec<MiNote>, ServerError> {
		let mut notes: Vec<MiNote> = Vec::new();
		let mut exclude_users = HashSet::new();
		for _ in 0..100 {
			let mut append_notes: Vec<MiNote> = match self.raw_htl(me_id, &opts, hints).await {
				Ok(v) => v,
				Err(e) => {
					if notes.is_empty() {
						return Err(e.into());
					} else {
						return Ok(notes);
					}
				}
			};
			if append_notes.is_empty() {
				return Ok(notes);
			}
			let _ = self
				.filter_note(
					&mut append_notes,
					Some(me_id),
					&mut exclude_users,
					hints,
					opts.with_cats,
				)
				.await?;
			if !append_notes.is_empty() {
				notes.extend_from_slice(&append_notes);
			}
			if !notes.is_empty() && (opts.allow_partial || notes.len() > opts.limit.into()) {
				return Ok(notes);
			}
		}
		Ok(notes)
	}
	/*全TL共通ミュート、ブロック、with_cats処理 */
	pub async fn filter_note(
		&self,
		notes: &mut Vec<MiNote>,
		me_id: Option<&String>,
		exclude_users: &mut HashSet<String>,
		hint: &mut TimelineHints,
		with_cats: bool,
	) -> Result<(), ServerError> {
		if notes.is_empty() {
			return Ok(());
		}
		use diesel::{ExpressionMethods, QueryDsl};
		use diesel_async::RunQueryDsl;
		let mut note_relation_user_ids = HashSet::new();
		for note in notes.iter() {
			if let Some(reply_id) = note.reply_id.as_ref() {
				if !hint.note_relation_note.contains_key(reply_id) {
					let reply =
						MiNote::load_by_id(&mut self.db.get_read_only().await?, reply_id).await?;
					//TLから除外するユーザーであればユーザー情報を取得する必要はない
					if !exclude_users.contains(&reply.user_id) {
						note_relation_user_ids.insert(reply.user_id.clone());
					}
					hint.note_relation_note.insert(reply_id.clone(), reply);
				}
			}
			if let Some(renote_id) = note.renote_id.as_ref() {
				if !hint.note_relation_note.contains_key(renote_id) {
					let renote =
						MiNote::load_by_id(&mut self.db.get_read_only().await?, renote_id).await?;
					if !exclude_users.contains(&renote.user_id) {
						note_relation_user_ids.insert(renote.user_id.clone());
					}
					hint.note_relation_note.insert(renote_id.clone(), renote);
				}
			}
			if !exclude_users.contains(&note.user_id) {
				note_relation_user_ids.insert(note.user_id.clone());
			}
		}
		if let Some(me_id) = me_id {
			let mut note_relation_user_ids: Vec<String> =
				note_relation_user_ids.into_iter().collect();
			//すでにミュート状態が明らかであれば取得対象から外す
			note_relation_user_ids.retain(|uid| !hint.is_muting_user.contains_key(uid));
			for (uid, is_mute) in &hint.is_muting_user {
				if *is_mute {
					if !exclude_users.contains(uid) {
						exclude_users.insert(uid.clone());
					}
				}
			}
			if !note_relation_user_ids.is_empty() {
				let f_block = async {
					use crate::models::blocking::blocking::dsl::blocking;
					use crate::models::blocking::blocking::dsl::*;
					let res: Result<Vec<String>, ServerError> = {
						let res = blocking
							.filter(blockerId.eq(me_id))
							.filter(blockeeId.eq_any(&note_relation_user_ids))
							.select(blockeeId)
							.load(&mut self.db.get_read_only().await?)
							.await?;
						Ok(res)
					};
					res
				};
				let f_mute = async {
					use crate::models::muting::muting::dsl::muting;
					use crate::models::muting::muting::dsl::*;
					let res: Result<Vec<String>, ServerError> = {
						let res = muting
							.filter(muterId.eq(me_id))
							.filter(muteeId.eq_any(&note_relation_user_ids))
							.select(muteeId)
							.load(&mut self.db.get_read_only().await?)
							.await?;
						Ok(res)
					};
					res
				};
				let (blocks, mutes) = futures_util::future::join(f_block, f_mute).await;
				let blocks = blocks?;
				let mutes = mutes?;
				for blockee_id in blocks {
					exclude_users.insert(blockee_id);
				}
				for mutee_id in mutes {
					exclude_users.insert(mutee_id);
				}
			}

			let followings_user = &mut hint.followings_user;
			let f_following = async {
				if followings_user.is_none() {
					followings_user.replace(self.followings(me_id).await?);
				}
				Ok::<&HashSet<std::string::String>, diesel::result::Error>(
					followings_user.as_ref().unwrap(),
				)
			};
			let renote_muting_user = &mut hint.renote_muting_user;
			let f_renote_muting = async {
				if renote_muting_user.is_none() {
					renote_muting_user.replace(self.renote_muting(me_id).await?);
				}
				Ok::<&HashSet<std::string::String>, diesel::result::Error>(
					renote_muting_user.as_ref().unwrap(),
				)
			};
			let hint_muted_instances = &mut hint.muted_instances;
			let f_muted_instances = async {
				if hint_muted_instances.is_none() {
					hint_muted_instances.replace(self.muted_instances(me_id).await?);
				}
				Ok::<&HashSet<std::string::String>, diesel::result::Error>(
					hint_muted_instances.as_ref().unwrap(),
				)
			};
			let (followings, renote_muting, muted_instances) =
				futures_util::future::join3(f_following, f_renote_muting, f_muted_instances).await;
			let followings = followings?;
			let renote_muting = renote_muting?;
			let muted_instances = muted_instances?;

			let filter = move |note: &MiNote| {
				if exclude_users.contains(&note.user_id) {
					return false;
				}
				if !note.is_visible(me_id, Some(&followings)) {
					return false;
				}
				if note.is_renote() && !note.is_quote() && renote_muting.contains(&note.user_id) {
					return false;
				}
				if let Some(user_host) = note.user_host.as_ref() {
					if muted_instances.contains(user_host) {
						return false;
					}
				}
				return true;
			};
			notes.retain(|note| {
				if !filter(&note) {
					return false;
				}
				if let Some(Some(reply)) = note
					.reply_id
					.as_ref()
					.map(|reply| hint.note_relation_note.get(reply))
				{
					if !filter(&reply) {
						return false;
					}
				}
				if let Some(Some(renote)) = note
					.renote_id
					.as_ref()
					.map(|renote| hint.note_relation_note.get(renote))
				{
					if !filter(&renote) {
						return false;
					}
				}
				return true;
			});
		}
		if with_cats {
			let user_ids: Vec<String> = notes
				.iter()
				.filter_map(|note| {
					if hint.user_cache.contains_key(&note.user_id) {
						None
					} else {
						Some(note.user_id.clone())
					}
				})
				.collect();
			if !user_ids.is_empty() {
				let append_users =
					MiUser::load_by_ids(&mut (&self.db).get_read_only().await?, &user_ids).await?;
				hint.user_cache
					.extend(append_users.into_iter().map(|user| (user.id.clone(), user)));
			}
			notes.retain(|note| {
				if let Some(user) = hint.user_cache.get(&note.user_id) {
					user.is_cat
				} else {
					false
				}
			});
		}
		Ok(())
	}
	async fn raw_stl(
		&self,
		me_id: &str,
		opt: &TLOptions,
		hint: &mut TimelineHints,
	) -> Result<Vec<MiNote>, ServerError> {
		use diesel::ExpressionMethods;
		use diesel::{QueryDsl, SelectableHelper};
		use diesel_async::RunQueryDsl;
		let followings_user = &mut hint.followings_user;
		let f_following = async {
			if followings_user.is_none() {
				followings_user.replace(self.followings(me_id).await?);
			}
			Ok::<&HashSet<std::string::String>, diesel::result::Error>(
				followings_user.as_ref().unwrap(),
			)
		};
		let hint_muted_instances = &mut hint.muted_instances;
		let f_muted_instances = async {
			if hint_muted_instances.is_none() {
				hint_muted_instances.replace(self.muted_instances(me_id).await?);
			}
			Ok::<&HashSet<std::string::String>, diesel::result::Error>(
				hint_muted_instances.as_ref().unwrap(),
			)
		};
		let (following_set, muted_instances) =
			futures_util::future::join(f_following, f_muted_instances).await;
		let mut following = following_set?.iter().collect::<Vec<_>>();
		let muted_instances = muted_instances?.iter().collect::<Vec<_>>();

		if opt.with_cats {
			//フォローユーザーでもcatではない事が明らかな場合は除外
			following.retain(|f| hint.user_cache.get(*f).map(|u| u.is_cat).unwrap_or(true));
		}

		let mut con = self.db.get_read_only().await?;
		let f_muting = async {
			use crate::models::muting::muting::dsl::muting;
			use crate::models::muting::muting::dsl::*;
			let res: Result<Vec<crate::models::muting::MiMuting>, diesel::result::Error> = muting
				.filter(muterId.eq(me_id))
				.filter(muteeId.eq_any(&following))
				.load(&mut con)
				.await;
			res
		};
		let muting = f_muting.await;
		for m in muting?.into_iter() {
			hint.is_muting_user.insert(m.mutee_id, true);
		}
		following.retain(|f| !*hint.is_muting_user.get(*f).unwrap_or(&false));
		for user_id in &following {
			hint.is_muting_user.insert((*user_id).clone(), false);
		}
		let me_id = me_id.to_string();
		following.push(&me_id); //自身をTLに含める
		let mut con = self.db.get_read_only().await?;
		let mut raw_tl: Vec<MiNote> = {
			use crate::models::note::note::dsl::note;
			use crate::models::note::note::dsl::*;
			use diesel::BoolExpressionMethods;
			let mut q = note
				.filter(
					userId.eq_any(&following).or(userHost
						.is_null()
						.and(visibility.eq(NoteVisibility::Public))),
				)
				.filter(userHost.ne_all(&muted_instances))
				.into_boxed();
			if !opt.with_renotes {
				q = q.filter(
					renoteId.is_null().or(text
						.is_not_null()
						//.or(fileIds.ne(Vec::<String>::new()))
						.or(cw.is_not_null())
						.or(replyId.is_not_null())
						.or(hasPoll.eq(true))),
				);
			}
			if opt.with_files {
				//TODO yojo-art 1.5.0時点ではfileIdsがVarChar[]型でdiesel側仕様でVarChar型が扱えない(Textとして扱われる)都合で型エラーを起こす
				//q = q.filter(fileIds.ne(Vec::<String>::new()));
			}
			if opt.since_id.is_some() && opt.until_id.is_none() {
				q = q.order(id.asc());
			} else {
				q = q.order(id.desc());
			}
			q = match (opt.since_id.as_ref(), opt.until_id.as_ref()) {
				(Some(since_id), Some(until_id)) => q.filter(id.between(since_id, until_id)),
				(Some(since_id), None) => q.filter(id.ge(since_id)),
				(None, Some(until_id)) => q.filter(id.le(until_id)),
				(None, None) => q,
			};
			q = q.limit(opt.limit.into());
			q.select(MiNote::as_select()).load(&mut con).await?
		};
		let remove_last = if let Some(note) = raw_tl.last() {
			Some(&note.id) == opt.since_id.as_ref() || Some(&note.id) == opt.until_id.as_ref()
		} else {
			false
		};
		if remove_last {
			raw_tl.remove(raw_tl.len() - 1);
		}
		let remove_first = if let Some(note) = raw_tl.get(0) {
			Some(&note.id) == opt.since_id.as_ref() || Some(&note.id) == opt.until_id.as_ref()
		} else {
			false
		};
		if remove_first {
			raw_tl.remove(0);
		}
		Ok(raw_tl)
	}
	async fn raw_ltl(
		&self,
		me_id: Option<&String>,
		opt: &TLOptions,
		hint: &mut TimelineHints,
	) -> Result<Vec<MiNote>, ServerError> {
		use diesel::ExpressionMethods;
		use diesel::{QueryDsl, SelectableHelper};
		use diesel_async::RunQueryDsl;
		let mut exclude_users = vec![];
		let muted_instances = if let Some(me_id) = me_id {
			let hint_muted_instances = &mut hint.muted_instances;
			let f_muted_instances = async {
				if hint_muted_instances.is_none() {
					hint_muted_instances.replace(self.muted_instances(me_id).await?);
				}
				Ok::<&HashSet<std::string::String>, diesel::result::Error>(
					hint_muted_instances.as_ref().unwrap(),
				)
			};
			let muted_instances = f_muted_instances.await;
			muted_instances?.iter().collect::<Vec<_>>()
		} else {
			Vec::new()
		};

		//with_cats処理かミュート処理が必要
		if opt.with_cats || me_id.is_some() {
			for user in hint.user_cache.values() {
				if user.host.is_some() {
					//リモートユーザーは元々除外
					continue;
				}
				//ミュートユーザーを除外
				if *hint.is_muting_user.get(&user.id).unwrap_or(&false) {
					exclude_users.push(&user.id);
					continue;
				}
				//catではない事が明らかな場合は除外
				if opt.with_cats && user.is_cat {
					exclude_users.push(&user.id);
				}
			}
		}

		let mut con = self.db.get_read_only().await?;
		let mut raw_tl: Vec<MiNote> = {
			use crate::models::note::note::dsl::note;
			use crate::models::note::note::dsl::*;
			use diesel::BoolExpressionMethods;
			let mut query = note
				.filter(userHost.is_null())
				.filter(visibility.eq(NoteVisibility::Public))
				.filter(userId.ne_all(&exclude_users))
				.filter(userHost.ne_all(&muted_instances))
				.into_boxed();
			if !opt.with_renotes {
				query = query.filter(
					renoteId.is_null().or(text
						.is_not_null()
						//.or(fileIds.ne(Vec::<String>::new()))
						.or(cw.is_not_null())
						.or(replyId.is_not_null())
						.or(hasPoll.eq(true))),
				);
			}
			if opt.with_files {
				//TODO yojo-art 1.5.0時点ではfileIdsがVarChar[]型でdiesel側仕様でVarChar型が扱えない(Textとして扱われる)都合で型エラーを起こす
				//q = q.filter(fileIds.ne(Vec::<String>::new()));
			}
			if opt.since_id.is_some() && opt.until_id.is_none() {
				query = query.order(id.asc());
			} else {
				query = query.order(id.desc());
			}
			query = match (opt.since_id.as_ref(), opt.until_id.as_ref()) {
				(Some(since_id), Some(until_id)) => query.filter(id.between(since_id, until_id)),
				(Some(since_id), None) => query.filter(id.ge(since_id)),
				(None, Some(until_id)) => query.filter(id.le(until_id)),
				(None, None) => query,
			};
			query = query.limit(opt.limit.into());
			query.select(MiNote::as_select()).load(&mut con).await?
		};
		let remove_last = if let Some(note) = raw_tl.last() {
			Some(&note.id) == opt.since_id.as_ref() || Some(&note.id) == opt.until_id.as_ref()
		} else {
			false
		};
		if remove_last {
			raw_tl.remove(raw_tl.len() - 1);
		}
		let remove_first = if let Some(note) = raw_tl.get(0) {
			Some(&note.id) == opt.since_id.as_ref() || Some(&note.id) == opt.until_id.as_ref()
		} else {
			false
		};
		if remove_first {
			raw_tl.remove(0);
		}
		Ok(raw_tl)
	}
	async fn raw_htl(
		&self,
		me_id: &str,
		opt: &TLOptions,
		hint: &mut TimelineHints,
	) -> Result<Vec<MiNote>, ServerError> {
		use diesel::ExpressionMethods;
		use diesel::{QueryDsl, SelectableHelper};
		use diesel_async::RunQueryDsl;
		let followings_user = &mut hint.followings_user;
		let f_following = async {
			if followings_user.is_none() {
				followings_user.replace(self.followings(me_id).await?);
			}
			Ok::<&HashSet<std::string::String>, diesel::result::Error>(
				followings_user.as_ref().unwrap(),
			)
		};
		let hint_muted_instances = &mut hint.muted_instances;
		let f_muted_instances = async {
			if hint_muted_instances.is_none() {
				hint_muted_instances.replace(self.muted_instances(me_id).await?);
			}
			Ok::<&HashSet<std::string::String>, diesel::result::Error>(
				hint_muted_instances.as_ref().unwrap(),
			)
		};
		let (following_set, muted_instances) =
			futures_util::future::join(f_following, f_muted_instances).await;
		let mut following = following_set?.iter().collect::<Vec<_>>();
		let muted_instances = muted_instances?.iter().collect::<Vec<_>>();

		if opt.with_cats {
			//フォローユーザーでもcatではない事が明らかな場合は除外
			following.retain(|f| hint.user_cache.get(*f).map(|u| u.is_cat).unwrap_or(true));
		}

		let mut con = self.db.get_read_only().await?;
		let f_muting = async {
			use crate::models::muting::muting::dsl::muting;
			use crate::models::muting::muting::dsl::*;
			let res: Result<Vec<crate::models::muting::MiMuting>, diesel::result::Error> = muting
				.filter(muterId.eq(me_id))
				.filter(muteeId.eq_any(&following))
				.load(&mut con)
				.await;
			res
		};
		let muting = f_muting.await;
		for m in muting?.into_iter() {
			hint.is_muting_user.insert(m.mutee_id, true);
		}
		following.retain(|f| !*hint.is_muting_user.get(*f).unwrap_or(&false));
		for user_id in &following {
			hint.is_muting_user.insert((*user_id).clone(), false);
		}
		let me_id = me_id.to_string();
		following.push(&me_id); //自身をTLに含める
		let mut con = self.db.get_read_only().await?;
		let mut raw_tl: Vec<MiNote> = {
			use crate::models::note::note::dsl::note;
			use crate::models::note::note::dsl::*;
			use diesel::BoolExpressionMethods;
			let mut q = note
				.filter(userId.eq_any(&following))
				.filter(userHost.ne_all(&muted_instances))
				.into_boxed();
			if !opt.with_renotes {
				q = q.filter(
					renoteId.is_null().or(text
						.is_not_null()
						//.or(fileIds.ne(Vec::<String>::new()))
						.or(cw.is_not_null())
						.or(replyId.is_not_null())
						.or(hasPoll.eq(true))),
				);
			}
			if opt.with_files {
				//TODO yojo-art 1.5.0時点ではfileIdsがVarChar[]型でdiesel側仕様でVarChar型が扱えない(Textとして扱われる)都合で型エラーを起こす
				//q = q.filter(fileIds.ne(Vec::<String>::new()));
			}
			if opt.since_id.is_some() && opt.until_id.is_none() {
				q = q.order(id.asc());
			} else {
				q = q.order(id.desc());
			}
			q = match (opt.since_id.as_ref(), opt.until_id.as_ref()) {
				(Some(since_id), Some(until_id)) => q.filter(id.between(since_id, until_id)),
				(Some(since_id), None) => q.filter(id.ge(since_id)),
				(None, Some(until_id)) => q.filter(id.le(until_id)),
				(None, None) => q,
			};
			q = q.limit(opt.limit.into());
			q.select(MiNote::as_select()).load(&mut con).await?
		};
		let remove_last = if let Some(note) = raw_tl.last() {
			Some(&note.id) == opt.since_id.as_ref() || Some(&note.id) == opt.until_id.as_ref()
		} else {
			false
		};
		if remove_last {
			raw_tl.remove(raw_tl.len() - 1);
		}
		let remove_first = if let Some(note) = raw_tl.get(0) {
			Some(&note.id) == opt.since_id.as_ref() || Some(&note.id) == opt.until_id.as_ref()
		} else {
			false
		};
		if remove_first {
			raw_tl.remove(0);
		}
		Ok(raw_tl)
	}
	async fn renote_muting(&self, me_id: &str) -> Result<HashSet<String>, diesel::result::Error> {
		let mut con = self.db.get_read_only().await.map_err(|e| {
			eprintln!("{}:{} {:?}", file!(), line!(), e);
			diesel::result::Error::BrokenTransactionManager
		})?;

		let mi_renote_muting: Vec<String> = {
			use crate::models::renote_muting::renote_muting::dsl::renote_muting;
			use crate::models::renote_muting::renote_muting::dsl::*;
			use diesel::{ExpressionMethods, QueryDsl};
			use diesel_async::RunQueryDsl;
			renote_muting
				.filter(muterId.eq(me_id))
				.select(muteeId)
				.load(&mut con)
				.await
		}?;
		Ok(to_set(mi_renote_muting.into_iter()))
	}

	async fn muted_instances(&self, me_id: &str) -> Result<HashSet<String>, diesel::result::Error> {
		let mut con = self.db.get_read_only().await.map_err(|e| {
			eprintln!("{}:{} {:?}", file!(), line!(), e);
			diesel::result::Error::BrokenTransactionManager
		})?;
		let res: MiUserProfile = {
			use crate::models::user_profile::user_profile::dsl::user_profile;
			use crate::models::user_profile::user_profile::dsl::*;
			use diesel::ExpressionMethods;
			use diesel::{QueryDsl, SelectableHelper};
			use diesel_async::RunQueryDsl;
			user_profile
				.filter(userId.eq(me_id))
				.select(MiUserProfile::as_select())
				.first(&mut con)
				.await
		}?;
		Ok(to_set(res.muted_instances.into_inner().into_iter()))
	}

	async fn followings(&self, me_id: &str) -> Result<HashSet<String>, diesel::result::Error> {
		let mut con = self.db.get_read_only().await.map_err(|e| {
			eprintln!("{}:{} {:?}", file!(), line!(), e);
			diesel::result::Error::BrokenTransactionManager
		})?;
		let mi_followings: Vec<String> = {
			use crate::models::following::following::dsl::following;
			use crate::models::following::following::dsl::*;
			use diesel::{ExpressionMethods, QueryDsl};
			use diesel_async::RunQueryDsl;
			following
				.filter(followerId.eq(me_id))
				.select(followeeId)
				.load(&mut con)
				.await
		}?;
		Ok(to_set(mi_followings.into_iter()))
	}
}
fn to_set<T>(v: impl Iterator<Item = T>) -> HashSet<T>
where
	T: std::hash::Hash,
	T: PartialEq,
	T: Eq,
{
	let mut s = HashSet::new();
	for f in v {
		s.insert(f);
	}
	s
}
