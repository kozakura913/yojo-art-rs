use std::{
	collections::{HashMap, HashSet},
	sync::Arc,
};

use redis::{AsyncCommands, aio::MultiplexedConnection};

use crate::{
	DBConnection, DataBase, MisskeyConfig, ParsedMisskeyConfig, ServerError,
	models::{
		blocking::MiBlocking, following::MiFollowing, muting::MiMuting, note::MiNote,
		renote_muting::MiRenoteMuting, user::MiUser, user_profile::MiUserProfile,
	},
};

use super::{
	event::EventService,
	id_service::IdService,
	meta::MetaService,
	note::{NoteService, PackedNote},
	role::RoleService,
	user::UserService,
};
pub enum FanoutTimelineName<'a> {
	Home(bool, &'a String),
	Local,
}
impl FanoutTimelineName<'_> {
	fn to_name(&self, host: impl AsRef<str>) -> String {
		match self {
			FanoutTimelineName::Home(with_files, user_id) => {
				if *with_files {
					format!("{}:list:homeTimelineWithFiles:{}", host.as_ref(), user_id)
				} else {
					format!("{}:list:homeTimeline:{}", host.as_ref(), user_id)
				}
			}
			FanoutTimelineName::Local => todo!(),
		}
	}
}
#[derive(Clone, Debug)]
pub struct FanoutTimelineService {
	config: Arc<ParsedMisskeyConfig>,
	db: DataBase,
	meta_service: MetaService,
	role_service: RoleService,
	id_service: IdService,
	user_service: UserService,
	event_service: EventService,
	redis_for_timelines: MultiplexedConnection,
	note_service: NoteService,
}
pub struct TLOptions {
	pub until_id: Option<String>,
	pub since_id: Option<String>,
	pub with_files: bool,
	pub with_renotes: bool,
	pub allow_partial: bool,
	pub with_cats: bool,
	pub limit: u16,
}
impl FanoutTimelineService {
	pub fn new(
		config: Arc<ParsedMisskeyConfig>,
		db: DataBase,
		meta_service: MetaService,
		role_service: RoleService,
		id_service: IdService,
		user_service: UserService,
		event_service: EventService,
		note_service: NoteService,
		redis_for_timelines: MultiplexedConnection,
	) -> Self {
		Self {
			config,
			db,
			meta_service,
			role_service,
			id_service,
			user_service,
			event_service,
			note_service,
			redis_for_timelines,
		}
	}
	pub async fn home_tl(
		&self,
		user_id: &String,
		opts: &TLOptions,
	) -> Result<Vec<PackedNote>, ServerError> {
		let mut con = self.db.get().await.ok_or("db error")?;
		let mut user_cache = HashMap::new();
		let (notes, relation_note) = self
			.get_notes(
				&mut con,
				Some(user_id),
				&FanoutTimelineName::Home(opts.with_files, user_id),
				&mut user_cache,
				opts,
			)
			.await?;
		let mut note_cache = HashMap::new();
		let mut packed_notes = vec![];
		for note in notes {
			let packed_note = self
				.note_service
				.pack_detail(
					&mut con,
					note,
					user_id,
					&mut user_cache,
					&mut note_cache,
					&relation_note,
				)
				.await?;
			packed_notes.push(packed_note);
		}
		Ok(packed_notes)
	}
	pub async fn get_notes(
		&self,
		con: &mut DBConnection<'_>,
		me_id: Option<&String>,
		timeline: &FanoutTimelineName<'_>,
		user_cache: &mut HashMap<String, MiUser>,
		opts: &TLOptions,
	) -> Result<(Vec<MiNote>, HashMap<String, MiNote>), ServerError> {
		let mut tl = self
			.redis_for_timelines
			.clone()
			.lrange::<String, Vec<String>>(timeline.to_name(&self.config.host), 0, -1)
			.await?;
		let ascending = opts.since_id.is_some() && opts.until_id.is_none();
		match (opts.since_id.as_ref(), opts.until_id.as_ref()) {
			(Some(since_id), Some(until_id)) => {
				tl.retain(|id| id < until_id && id > since_id);
			}
			(None, Some(until_id)) => {
				tl.retain(|id| id < until_id);
			}
			(Some(since_id), None) => {
				tl.retain(|id| id > since_id);
			}
			(None, None) => {}
		};
		if ascending {
			tl.sort_by(|a, b| a.cmp(b));
		} else {
			tl.sort_by(|a, b| b.cmp(a));
		}
		use diesel::ExpressionMethods;
		use diesel::{QueryDsl, SelectableHelper};
		use diesel_async::RunQueryDsl;
		use tokio::sync::RwLock;

		use crate::{DataBase, models::note::MiNote};
		let mut notes: Vec<MiNote> = Vec::new();
		let mut note_relation_note = HashMap::new();
		let mut exclude_users = HashSet::new();
		while !tl.is_empty() {
			let limit_tl: Vec<String> = tl.drain(0..tl.len().min(opts.limit as usize)).collect();
			let mut append_notes: Vec<MiNote> = {
				use crate::models::note::note::dsl::note;
				use crate::models::note::note::dsl::*;
				note.filter(id.eq_any(&limit_tl))
					.select(MiNote::as_select())
					.load(con)
					.await?
			};
			append_notes.retain(|note| opts.with_renotes || !note.is_renote() || note.is_quote());
			let _ = self
				.filter_note(
					con,
					&mut append_notes,
					me_id,
					&mut note_relation_note,
					&mut exclude_users,
				)
				.await?;
			if !append_notes.is_empty() {
				if opts.with_cats {
					let user_ids: Vec<String> = append_notes
						.iter()
						.filter_map(|note| {
							if user_cache.contains_key(&note.user_id) {
								None
							} else {
								Some(note.user_id.clone())
							}
						})
						.collect();
					if !user_ids.is_empty() {
						let append_users = MiUser::load_by_ids(con, &user_ids).await?;
						user_cache
							.extend(append_users.into_iter().map(|user| (user.id.clone(), user)));
					}
					append_notes.retain(|note| {
						if let Some(user) = user_cache.get(&note.user_id) {
							user.is_cat
						} else {
							false
						}
					});
				}
				notes.extend_from_slice(&append_notes);
			}
			if !notes.is_empty() && (opts.allow_partial || notes.len() > opts.limit.into()) {
				break;
			}
		}
		notes.sort_by(|a, b| {
			if ascending {
				a.id.cmp(&b.id)
			} else {
				b.id.cmp(&a.id)
			}
		});
		notes.truncate(opts.limit as usize);
		Ok((notes, note_relation_note))
	}

	async fn filter_note(
		&self,
		con: &mut DBConnection<'_>,
		notes: &mut Vec<MiNote>,
		me_id: Option<&String>,
		note_relation_note: &mut HashMap<String, MiNote>,
		exclude_users: &mut HashSet<String>,
	) -> Result<(), ServerError> {
		if notes.is_empty() {
			return Ok(());
		}
		use diesel::ExpressionMethods;
		use diesel::{QueryDsl, SelectableHelper};
		use diesel_async::RunQueryDsl;
		let mut note_relation_user_ids = HashSet::new();
		for note in notes.iter() {
			if let Some(reply_id) = note.reply_id.as_ref() {
				let reply = MiNote::load_by_id(con, reply_id).await?;
				if !exclude_users.contains(&reply.user_id) {
					note_relation_user_ids.insert(reply.user_id.clone());
				}
				note_relation_note.insert(reply_id.clone(), reply);
			}
			if let Some(renote_id) = note.renote_id.as_ref() {
				let renote = MiNote::load_by_id(con, renote_id).await?;
				if !exclude_users.contains(&renote.user_id) {
					note_relation_user_ids.insert(renote.user_id.clone());
				}
				note_relation_note.insert(renote_id.clone(), renote);
			}
			if !exclude_users.contains(&note.user_id) {
				note_relation_user_ids.insert(note.user_id.clone());
			}
		}
		if let Some(me_id) = me_id {
			let note_relation_user_ids: Vec<String> = note_relation_user_ids.into_iter().collect();
			if !note_relation_user_ids.is_empty() {
				use crate::models::blocking::blocking::dsl::blocking;
				use crate::models::blocking::blocking::dsl::*;
				let res: Vec<MiBlocking> = blocking
					.filter(blockerId.eq(me_id))
					.filter(blockeeId.eq_any(&note_relation_user_ids))
					.select(MiBlocking::as_select())
					.load(con)
					.await?;
				for block in res {
					exclude_users.insert(block.blockee_id);
				}
			}
			{
				use crate::models::muting::muting::dsl::muting;
				use crate::models::muting::muting::dsl::*;
				let res: Vec<MiMuting> = muting
					.filter(muterId.eq(me_id))
					.filter(muteeId.eq_any(&note_relation_user_ids))
					.select(MiMuting::as_select())
					.load(con)
					.await?;
				for mute in res {
					exclude_users.insert(mute.mutee_id);
				}
			}

			let mi_followings: Vec<MiFollowing> = {
				use crate::models::following::following::dsl::following;
				use crate::models::following::following::dsl::*;
				following
					.filter(followerId.eq(me_id))
					.select(MiFollowing::as_select())
					.load(con)
					.await?
			};
			let mut followings = HashSet::new();
			for f in mi_followings {
				followings.insert(f.followee_id);
			}
			let muted_instances = MiUserProfile::load_by_user(con, &me_id)
				.await
				.map(|profile| {
					let mut muted_instances = HashSet::new();
					for host in profile.muted_instances.into_inner() {
						muted_instances.insert(host);
					}
					muted_instances
				});

			let renote_muting = {
				use crate::models::renote_muting::renote_muting::dsl::renote_muting;
				use crate::models::renote_muting::renote_muting::dsl::*;
				let mi_renote_muting: Vec<MiRenoteMuting> = renote_muting
					.filter(muterId.eq(me_id))
					.select(MiRenoteMuting::as_select())
					.load(con)
					.await?;
				let mut renote_muting_set = HashSet::new();
				for renote_mute in mi_renote_muting {
					renote_muting_set.insert(renote_mute.mutee_id);
				}
				renote_muting_set
			};
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
				if let Some((muted_instances, user_host)) =
					muted_instances.as_ref().zip(note.user_host.as_ref())
				{
					if muted_instances.contains(user_host) {
						return false;
					}
				}
				//mi_renote_muting
				return true;
			};
			notes.retain(|note| {
				if !filter(&note) {
					return false;
				}
				if let Some(Some(reply)) = note
					.reply_id
					.as_ref()
					.map(|reply| note_relation_note.get(reply))
				{
					if !filter(&reply) {
						return false;
					}
				}
				if let Some(Some(renote)) = note
					.renote_id
					.as_ref()
					.map(|renote| note_relation_note.get(renote))
				{
					if !filter(&renote) {
						return false;
					}
				}
				return true;
			});
		}
		Ok(())
	}
}
