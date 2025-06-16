use std::{
	borrow::Cow,
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
	service::timeline::{TLOptions, TimelineHints, TimelineService},
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
	timeline_service: TimelineService,
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
		timeline_service: TimelineService,
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
			timeline_service,
		}
	}
	pub async fn home_tl(
		&self,
		user_id: &String,
		opts: &TLOptions,
	) -> Result<Vec<PackedNote>, ServerError> {
		let mut con = self.db.get_read_only().await?;
		let mut user_cache = HashMap::new();
		let (mut notes, mut relation_note) = self
			.get_notes(
				&mut con,
				Some(user_id),
				&FanoutTimelineName::Home(opts.with_files, user_id),
				&mut user_cache,
				opts,
			)
			.await?;
		let meta = self.meta_service.load(true).await.ok_or("db meta")?;
		if meta.other.enable_fanout_timeline_db_fallback
			&& (notes.is_empty() || (!opts.allow_partial && (notes.len() <= opts.limit.into())))
		{
			let opts = if let Some(last) = notes.last() {
				let mut opts = opts.clone();
				if opts.since_id.is_some() && opts.until_id.is_none() {
					opts.since_id = Some(last.id.clone());
				} else {
					opts.until_id = Some(last.id.clone());
				}
				Cow::Owned(opts)
			} else {
				Cow::Borrowed(opts)
			};
			let (add_notes, add_relation_note) = self
				.timeline_service
				.get_htl(user_id, &mut user_cache, &opts)
				.await?;
			relation_note.extend(add_relation_note);
			notes.extend_from_slice(&add_notes);
		}
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
		let mut exclude_users = HashSet::new();
		let mut hints = TimelineHints {
			..Default::default()
		};
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
				.timeline_service
				.filter_note(
					con,
					&mut append_notes,
					me_id,
					&mut exclude_users,
					&mut hints,
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
		Ok((notes, hints.note_relation_note))
	}
}
