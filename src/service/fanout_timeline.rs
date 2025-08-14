use std::{borrow::Cow, collections::HashSet, sync::Arc};

use redis::{AsyncCommands, aio::MultiplexedConnection};

use crate::{
	DataBase, ParsedMisskeyConfig, ServerError,
	models::{note::MiNote, user::MiUser},
	service::timeline::{TLOptions, TimelineHints, TimelineService},
};

use super::meta::MetaService;
pub enum FanoutTimelineName<'a> {
	Home(bool, &'a String),
	Local(bool, bool, Option<&'a String>),
	Hybrid(bool, bool, &'a String),
}
impl FanoutTimelineName<'_> {
	fn to_names(&self, host: impl AsRef<str>) -> Vec<String> {
		match self {
			FanoutTimelineName::Home(with_files, user_id) => {
				if *with_files {
					vec![format!(
						"{}:list:homeTimelineWithFiles:{}",
						host.as_ref(),
						user_id
					)]
				} else {
					vec![format!("{}:list:homeTimeline:{}", host.as_ref(), user_id)]
				}
			}
			FanoutTimelineName::Local(with_files, with_replies, user_id) => {
				if *with_files {
					vec![format!("{}:list:localTimelineWithFiles", host.as_ref())]
				} else if *with_replies {
					vec![
						format!("{}:list:localTimeline", host.as_ref()),
						format!("{}:list:localTimelineWithReplies", host.as_ref()),
					]
				} else if let Some(user_id) = user_id {
					vec![
						format!("{}:list:localTimeline", host.as_ref()),
						format!(
							"{}:list:localTimelineWithReplyTo:{}",
							host.as_ref(),
							*user_id
						),
					]
				} else {
					vec![format!("{}:list:localTimeline", host.as_ref())]
				}
			}
			FanoutTimelineName::Hybrid(with_files, with_replies, user_id) => {
				if *with_files {
					vec![
						format!("{}:list:homeTimelineWithFiles:{}", host.as_ref(), user_id),
						format!("{}:list:localTimelineWithFiles", host.as_ref()),
					]
				} else if *with_replies {
					vec![
						format!("{}:list:homeTimeline:{}", host.as_ref(), user_id),
						format!("{}:list:localTimeline", host.as_ref()),
						format!("{}:list:localTimelineWithReplies", host.as_ref()),
					]
				} else {
					vec![
						format!("{}:list:homeTimeline:{}", host.as_ref(), user_id),
						format!("{}:list:localTimeline", host.as_ref()),
						format!("{}:list:localTimelineWithReplyTo{}", host.as_ref(), user_id),
					]
				}
			}
		}
	}
}
#[derive(Clone, Debug)]
pub struct FanoutTimelineService {
	config: Arc<ParsedMisskeyConfig>,
	db: DataBase,
	meta_service: MetaService,
	redis_for_timelines: MultiplexedConnection,
	timeline_service: TimelineService,
}
impl FanoutTimelineService {
	pub fn new(
		config: Arc<ParsedMisskeyConfig>,
		db: DataBase,
		meta_service: MetaService,
		redis_for_timelines: MultiplexedConnection,
		timeline_service: TimelineService,
	) -> Self {
		Self {
			config,
			db,
			meta_service,
			redis_for_timelines,
			timeline_service,
		}
	}
	pub async fn get_stl(
		&self,
		user_id: &String,
		hints: &mut TimelineHints,
		opts: &TLOptions,
	) -> Result<Vec<MiNote>, ServerError> {
		let mut notes = self
			.get_notes(
				Some(user_id),
				&&FanoutTimelineName::Hybrid(opts.with_files, opts.with_replies, user_id),
				opts,
				hints,
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
			let add_notes = self.timeline_service.get_stl(user_id, hints, &opts).await?;
			notes.extend_from_slice(&add_notes);
		}
		Ok(notes)
	}
	pub async fn get_ltl(
		&self,
		user_id: Option<&String>,
		hints: &mut TimelineHints,
		opts: &TLOptions,
	) -> Result<Vec<MiNote>, ServerError> {
		let mut notes = self
			.get_notes(
				user_id,
				&FanoutTimelineName::Local(opts.with_files, opts.with_replies, user_id),
				opts,
				hints,
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
			let add_notes = self.timeline_service.get_ltl(user_id, hints, &opts).await?;
			notes.extend_from_slice(&add_notes);
		}
		Ok(notes)
	}
	pub async fn get_htl(
		&self,
		user_id: &String,
		hints: &mut TimelineHints,
		opts: &TLOptions,
	) -> Result<Vec<MiNote>, ServerError> {
		let mut notes = self
			.get_notes(
				Some(user_id),
				&FanoutTimelineName::Home(opts.with_files, user_id),
				opts,
				hints,
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
			let add_notes = self.timeline_service.get_htl(user_id, hints, &opts).await?;
			notes.extend_from_slice(&add_notes);
		}
		Ok(notes)
	}
	pub async fn get_notes(
		&self,
		me_id: Option<&String>,
		timeline: &FanoutTimelineName<'_>,
		opts: &TLOptions,
		hints: &mut TimelineHints,
	) -> Result<Vec<MiNote>, ServerError> {
		let ascending = opts.since_id.is_some() && opts.until_id.is_none();
		let mut merge_tl = vec![];
		for name in timeline.to_names(&self.config.host) {
			let mut tl = self
				.redis_for_timelines
				.clone()
				.lrange::<String, Vec<String>>(name, 0, -1)
				.await?;
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
			merge_tl.extend_from_slice(&tl);
		}

		let mut notes: Vec<MiNote> = Vec::new();
		let mut exclude_users = HashSet::new();
		while !merge_tl.is_empty() {
			let limit_tl: Vec<String> = merge_tl
				.drain(0..merge_tl.len().min(opts.limit as usize))
				.collect();
			let mut append_notes =
				MiNote::load_by_ids(&mut self.db.get_read_only().await?, limit_tl.iter()).await?;
			append_notes.retain(|note| opts.with_renotes || !note.is_renote() || note.is_quote());
			let _ = self
				.timeline_service
				.filter_note(
					&mut append_notes,
					me_id,
					&mut exclude_users,
					hints,
					opts.with_cats,
				)
				.await?;
			if !append_notes.is_empty() {
				if opts.with_cats {
					let user_ids: Vec<String> = append_notes
						.iter()
						.filter_map(|note| {
							if hints.user_cache.contains_key(&note.user_id) {
								None
							} else {
								Some(note.user_id.clone())
							}
						})
						.collect();
					if !user_ids.is_empty() {
						let append_users = MiUser::load_by_ids(
							&mut self.db.get_read_only().await?,
							user_ids.iter(),
						)
						.await?;
						hints
							.user_cache
							.extend(append_users.into_iter().map(|user| (user.id.clone(), user)));
					}
					append_notes.retain(|note| {
						if let Some(user) = hints.user_cache.get(&note.user_id) {
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
		Ok(notes)
	}
}
