use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use memory_cache::MemoryCache;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{
	DBConnection, DataBase, MisskeyConfig, ServerError,
	models::{
		self,
		common::NoteSearchableBy,
		drive_file::{FileProperties, MiDriveFile},
		drive_folder::MiDriveFolder,
		event::{EventMetadata, MiEvent},
		note::{MiNote, MiReactions, NoteReactionAcceptances, NoteVisibility},
		note_reaction::MiNoteReaction,
		poll::MiPoll,
		poll_vote::MiPollVote,
		user::MiUser,
		user_profile::MiUserProfile,
	},
	service::event::{DriveEventType, MainEventType},
};

use super::{
	drive::DriveService,
	emoji::EmojiService,
	event::EventService,
	id_service::IdService,
	meta::MetaService,
	role::RoleService,
	user::{PackedUserLite, UserService},
};
#[derive(Clone, Debug)]
pub struct NoteService {
	config: Arc<MisskeyConfig>,
	db: DataBase,
	meta_service: MetaService,
	role_service: RoleService,
	drive_service: DriveService,
	id_service: IdService,
	user_service: UserService,
	emoji_service: EmojiService,
	event_service: EventService,
}

impl NoteService {
	pub fn new(
		config: Arc<MisskeyConfig>,
		db: DataBase,
		meta_service: MetaService,
		role_service: RoleService,
		drive_service: DriveService,
		id_service: IdService,
		user_service: UserService,
		emoji_service: EmojiService,
		event_service: EventService,
	) -> Self {
		Self {
			config,
			db,
			meta_service,
			role_service,
			drive_service,
			id_service,
			user_service,
			emoji_service,
			event_service,
		}
	}
	pub async fn pack_detail_many(
		&self,
		notes: impl Iterator<Item = MiNote>,
		me_id: Option<&String>,
		note_hint: &HashMap<String, MiNote>,
	) -> Vec<PackedNote> {
		let user_cache = Arc::new(RwLock::new(HashMap::new()));
		let note_cache = Arc::new(RwLock::new(HashMap::new()));
		let mut f_packed_notes = vec![];
		for note in notes {
			let packed_note = self.pack_detail(
				note,
				me_id,
				user_cache.clone(),
				note_cache.clone(),
				&note_hint,
			);
			f_packed_notes.push(packed_note);
		}
		let mut packed_notes = Vec::with_capacity(f_packed_notes.len());
		for packed in futures_util::future::join_all(f_packed_notes)
			.await
			.into_iter()
		{
			match packed {
				Ok(note) => packed_notes.push(note),
				Err(e) => {
					eprintln!("{}:{} {}", file!(), line!(), e.text);
				}
			}
		}
		packed_notes
	}
	pub async fn pack_detail(
		&self,
		note: MiNote,
		me_id: Option<&String>,
		user_cache: Arc<RwLock<HashMap<String, MiUser>>>,
		note_cache: Arc<RwLock<HashMap<String, PackedNote>>>,
		note_hint: &HashMap<String, MiNote>,
	) -> Result<PackedNote, ServerError> {
		let reply_id = note.reply_id.clone();
		let user_cache0 = user_cache.clone();
		let reply = async {
			match reply_id {
				Some(reply_id) => {
					let note = self
						.cache_or_pack(&reply_id, me_id, user_cache0, note_cache.clone(), note_hint)
						.await;
					note.map(|note| Some(Box::new(note)))
				}
				None => Ok(None),
			}
		};
		let renote_id = note.renote_id.clone();
		let user_cache0 = user_cache.clone();
		let renote = async {
			match renote_id {
				Some(renote_id) => {
					let note = self
						.cache_or_pack(
							&renote_id,
							me_id,
							user_cache0,
							note_cache.clone(),
							note_hint,
						)
						.await;
					note.map(|note| Some(Box::new(note)))
				}
				None => Ok(None),
			}
		};
		let (packed_note, renote, reply) =
			futures_util::future::join3(self.pack(note, me_id, user_cache), renote, reply).await;
		let mut packed_note = packed_note?;
		packed_note.renote = renote?;
		packed_note.reply = reply?;
		Ok(packed_note)
	}
	async fn cache_or_pack(
		&self,
		note_id: &String,
		me_id: Option<&String>,
		user_cache: Arc<RwLock<HashMap<String, MiUser>>>,
		note_cache: Arc<RwLock<HashMap<String, PackedNote>>>,
		note_hint: &HashMap<String, MiNote>,
	) -> Result<PackedNote, ServerError> {
		{
			let read_lock = note_cache.read().await;
			if let Some(note) = read_lock.get(note_id) {
				return Ok(note.clone());
			}
		}
		let mut write_lock = note_cache.write().await;
		let note = match note_hint.get(note_id) {
			Some(note) => note.clone(),
			None => MiNote::load_by_id(&mut self.db.get_read_only().await?, note_id).await?,
		};
		let packed = self.pack(note, me_id, user_cache).await?;
		write_lock.insert(packed.id.clone(), packed.clone());
		Ok(packed)
	}
	/* renoteとreplyを処理しない */
	pub async fn pack(
		&self,
		note: MiNote,
		me_id: Option<&String>,
		user_cache: Arc<RwLock<HashMap<String, MiUser>>>,
	) -> Result<PackedNote, ServerError> {
		let user = if let Some(u) = user_cache.read().await.get(&note.user_id).cloned() {
			u
		} else {
			let mut w_lock = user_cache.write().await;
			let u = MiUser::load_by_id(&mut self.db.get_read_only().await?, &note.user_id).await?;
			w_lock.insert(note.user_id.clone(), u.clone());
			u
		};
		let visible_user_ids = if note.visibility == NoteVisibility::Specified {
			Some(note.visible_user_ids)
		} else {
			None
		};
		let mut reactions = note.reactions;
		reactions.0.retain(|_, count| count.is_positive());
		let reaction_count = reactions.0.values().fold(0, |a, b| a + *b);
		let reaction_emoji_names = reactions
			.0
			.keys()
			.into_iter()
			.map(|emoji_name| {
				if emoji_name.len() > 2 {
					let mut chars = emoji_name.chars();
					if chars.next() == Some(':') && chars.rev().next() == Some(':') {
						return (&emoji_name[1..emoji_name.len() - 1]).to_owned();
					}
				}
				emoji_name.to_owned()
			})
			.collect();
		let un_normalize_reactions = reactions;
		let mut reactions = MiReactions(HashMap::new());
		for (k, v) in un_normalize_reactions.0.into_iter() {
			let k = self.emoji_service.normalize_reaction(k);
			reactions.0.insert(k, v);
		}
		let files = if note.file_ids.is_empty() {
			vec![]
		} else {
			let files: Vec<MiDriveFile> = {
				use crate::models::drive_file::drive_file::dsl::drive_file;
				use crate::models::drive_file::drive_file::dsl::*;
				use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
				use diesel_async::RunQueryDsl;
				drive_file
					.filter(id.eq_any(&note.file_ids))
					.select(MiDriveFile::as_select())
					.load(&mut self.db.get_read_only().await?)
					.await
			}?;
			let mut packed_files = Vec::new();
			for f in files.iter() {
				let is_my_file = me_id.is_some() && f.user_id.as_ref() == me_id;
				let packed = self
					.drive_service
					.pack(
						&mut self.db.get_read_only().await?,
						f,
						is_my_file,
						false,
						false,
						None,
						Some(&user),
					)
					.await;
				packed_files.push(packed.ok_or("pack file")?);
			}
			packed_files
		};
		let emojis = if let Some(host) = user.host.as_ref() {
			let mut emojis = note.emojis;
			for emoji_name in emojis.iter_mut() {
				if emoji_name.len() > 2 {
					let mut chars = emoji_name.chars();
					if chars.next() == Some(':') && chars.rev().next() == Some(':') {
						*emoji_name = (&emoji_name[1..emoji_name.len() - 1]).to_owned();
					}
				}
			}
			Some(
				self.emoji_service
					.populate_emojis(emojis, Some(host.clone()))
					.await,
			)
		} else {
			None
		};
		let reaction_emojis = self
			.emoji_service
			.populate_emojis(reaction_emoji_names, user.host.clone())
			.await;
		let event = if note.has_event {
			self.populate_event(&note.id).await
		} else {
			None
		};
		let poll = if note.has_poll {
			self.populate_poll(&note.id, me_id).await
		} else {
			None
		};
		let user = self.user_service.pack_lite(user.clone()).await?;
		let created_at = self.id_service.parse(&note.id).ok_or("parse id")?;
		let my_reaction = if reaction_count < 1 || me_id.is_none() {
			None
		} else if reaction_count as usize <= note.reaction_and_user_pair_cache.len() {
			let mut reactions: Vec<String> = Vec::new();
			for cache in note.reaction_and_user_pair_cache.iter() {
				if !cache.starts_with(me_id.unwrap()) {
					continue;
				}
				let mut split = cache.split('/');
				split.next(); //user_id
				if let Some(src) = split.next() {
					let parsed = self.emoji_service.normalize_reaction(src.to_owned());
					reactions.push(parsed);
				}
			}
			reactions.into_iter().next()
		} else {
			// 作成直後はリアクションが無いと思われるのでコストの高いDBクエリしない
			if created_at > Utc::now() - Duration::seconds(2) {
				None
			} else {
				let reactions: Vec<MiNoteReaction> = {
					use crate::models::note_reaction::note_reaction::dsl::note_reaction;
					use crate::models::note_reaction::note_reaction::dsl::*;
					use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
					use diesel_async::RunQueryDsl;
					note_reaction
						.filter(noteId.eq(&note.id))
						.filter(userId.eq(&me_id.unwrap()))
						.select(MiNoteReaction::as_select())
						.load(&mut self.db.get_read_only().await?)
						.await
				}?;
				let reactions: Vec<String> =
					reactions.into_iter().map(|react| react.reaction).collect();
				println!("my reactions from db {:?}", reactions);
				let emoji = reactions.into_iter().next();
				emoji.map(|src| self.emoji_service.normalize_reaction(src))
			}
		};
		let mut packed_note = PackedNote {
			created_at,
			updated_at: note.updated_at.as_ref().map(|time| time.and_utc()),
			updated_at_history: note.updated_at_history.as_ref().map(|v| {
				use std::iter::Iterator;
				v.iter()
					.map(|time| time.and_utc())
					.collect::<Vec<DateTime<Utc>>>()
			}),
			delete_at: note.delete_at.as_ref().map(|time| time.and_utc()),
			user,
			user_id: note.user_id,
			cw: note.cw,
			text: note.text,
			visibility: note.visibility,
			searchable_by: note.searchable_by,
			local_only: note.local_only,
			reaction_acceptance: note.reaction_acceptance,
			visible_user_ids,
			disable_right_click: if note.disable_right_click {
				Some(true)
			} else {
				None
			},
			renote_count: note.renote_count,
			replies_count: note.replies_count,
			reaction_count,
			reactions,
			reaction_emojis,
			emojis,
			tags: if note.tags.is_empty() {
				None
			} else {
				Some(note.tags)
			},
			file_ids: note.file_ids,
			files,
			reply_id: note.reply_id,
			renote_id: note.renote_id,
			mentions: if note.mentions.is_empty() {
				None
			} else {
				Some(note.mentions)
			},
			id: note.id,
			clipped_count: note.clipped_count,
			reply: None,  //pack_detailで埋める
			renote: None, //pack_detailで埋める
			event,
			poll,
			my_reaction,
		};
		self.treat_visibility(&mut packed_note)?;
		Ok(packed_note)
	}
	pub async fn populate_poll(
		&self,
		note_id: &String,
		me_id: Option<&String>,
	) -> Option<PackedPoll> {
		let mut con = self
			.db
			.get_read_only()
			.await
			.map_err(|e| {
				eprintln!("{}:{} {:?}", file!(), line!(), e);
			})
			.ok()?;
		let mi_poll: MiPoll = {
			use crate::models::poll::poll::dsl::noteId;
			use crate::models::poll::poll::dsl::poll;
			use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
			use diesel_async::RunQueryDsl;
			poll.filter(noteId.eq(note_id))
				.select(MiPoll::as_select())
				.first(&mut con)
				.await
				.map_err(|e| {
					eprintln!("{}:{} {:?}", file!(), line!(), e);
				})
				.ok()
		}?;
		let mut choices: Vec<PackedPollChoices> = mi_poll
			.choices
			.into_iter()
			.zip(mi_poll.votes.into_iter())
			.map(|(text, votes)| PackedPollChoices {
				text,
				votes,
				is_voted: false,
			})
			.collect();
		if let Some(me_id) = me_id {
			let total = choices.iter().fold(0, |count, c| count + c.votes);
			let votes: Option<Vec<MiPollVote>> = if total <= 0 {
				None
			} else {
				use crate::models::poll_vote::poll_vote::dsl::poll_vote;
				use crate::models::poll_vote::poll_vote::dsl::{noteId, userId};
				use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
				use diesel_async::RunQueryDsl;
				poll_vote
					.filter(noteId.eq(note_id))
					.filter(userId.eq(me_id))
					.select(MiPollVote::as_select())
					.load(&mut con)
					.await
					.map_err(|e| {
						eprintln!("{}:{} {:?}", file!(), line!(), e);
					})
					.ok()
			};
			if let Some(mut votes) = votes {
				if !mi_poll.multiple {
					votes.truncate(1);
				}
				if !votes.is_empty() {
					for choice in votes.into_iter().map(|v| v.choice) {
						if choice > 0 {
							if let Some(c) = choices.get_mut(choice as usize) {
								c.is_voted = true;
							}
						}
					}
				}
			}
		}
		Some(PackedPoll {
			multiple: mi_poll.multiple,
			expires_at: mi_poll.expires_at.map(|t| t.and_utc()),
			choices,
		})
	}
	pub async fn populate_event(&self, note_id: &String) -> Option<PackedEvent> {
		let mut con = self
			.db
			.get_read_only()
			.await
			.map_err(|e| {
				eprintln!("{}:{} {:?}", file!(), line!(), e);
			})
			.ok()?;
		let event: MiEvent = {
			use crate::models::event::event::dsl::event;
			use crate::models::event::event::dsl::noteId;
			use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
			use diesel_async::RunQueryDsl;
			event
				.filter(noteId.eq(note_id))
				.select(MiEvent::as_select())
				.first(&mut con)
				.await
				.map_err(|e| {
					eprintln!("{}:{} {:?}", file!(), line!(), e);
				})
				.ok()
		}?;
		Some(PackedEvent {
			title: event.title,
			start: event.start.and_utc(),
			end: event.end.map(|t| t.and_utc()),
			metadata: event.metadata,
		})
	}
	pub fn treat_visibility(&self, packed_note: &mut PackedNote) -> Result<(), ServerError> {
		use NoteVisibility::*;
		match packed_note.visibility {
			Public | Home => {
				if let Some(followers_only_before) =
					packed_note.user.make_notes_followers_only_before
				{
					let created_at = self
						.id_service
						.parse(&packed_note.id)
						.ok_or("parse created_at")?;
					if followers_only_before <= 0 {
						if (Utc::now() - created_at).num_milliseconds()
							> 0 - (followers_only_before as i64 * 1000)
						{
							packed_note.visibility = Followers;
						}
					} else {
						if created_at.timestamp() < followers_only_before as i64 {
							packed_note.visibility = Followers;
						}
					}
				}
			}
			_ => {}
		}
		Ok(())
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackedNote {
	id: String,
	#[serde(rename = "createdAt")]
	created_at: DateTime<Utc>,
	#[serde(rename = "updatedAt")]
	#[serde(skip_serializing_if = "Option::is_none")]
	updated_at: Option<DateTime<Utc>>,
	#[serde(rename = "updatedAtHistory")]
	#[serde(skip_serializing_if = "Option::is_none")]
	updated_at_history: Option<Vec<DateTime<Utc>>>,
	//noteEditHistory
	#[serde(rename = "deleteAt")]
	delete_at: Option<DateTime<Utc>>,
	user: PackedUserLite,
	#[serde(rename = "userId")]
	user_id: String,
	text: Option<String>,
	cw: Option<String>,
	visibility: NoteVisibility,
	#[serde(rename = "searchableBy")]
	searchable_by: Option<NoteSearchableBy>,
	#[serde(rename = "localOnly")]
	local_only: bool,
	#[serde(rename = "reactionAcceptance")]
	reaction_acceptance: Option<NoteReactionAcceptances>,
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "visibleUserIds")]
	visible_user_ids: Option<Vec<String>>,
	#[serde(rename = "disableRightClick")]
	#[serde(skip_serializing_if = "Option::is_none")]
	disable_right_click: Option<bool>,
	#[serde(rename = "renoteCount")]
	renote_count: i16,
	#[serde(rename = "repliesCount")]
	replies_count: i16,
	#[serde(rename = "reactionCount")]
	reaction_count: i32,
	reactions: MiReactions,
	#[serde(rename = "reactionEmojis")]
	reaction_emojis: HashMap<String, String>, //id:url
	#[serde(skip_serializing_if = "Option::is_none")]
	emojis: Option<HashMap<String, String>>, //id:url
	#[serde(skip_serializing_if = "Option::is_none")]
	tags: Option<Vec<String>>,
	#[serde(rename = "fileIds")]
	file_ids: Vec<String>,
	files: Vec<serde_json::Value>,
	#[serde(rename = "replyId")]
	reply_id: Option<String>,
	#[serde(rename = "renoteId")]
	renote_id: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	mentions: Option<Vec<String>>,
	#[serde(rename = "clippedCount")]
	clipped_count: i16,
	#[serde(skip_serializing_if = "Option::is_none")]
	reply: Option<Box<PackedNote>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	renote: Option<Box<PackedNote>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	event: Option<PackedEvent>,
	poll: Option<PackedPoll>,
	#[serde(rename = "myReaction")]
	#[serde(skip_serializing_if = "Option::is_none")]
	my_reaction: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackedEvent {
	title: String,
	start: DateTime<Utc>,
	end: Option<DateTime<Utc>>,
	metadata: EventMetadata,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackedPoll {
	multiple: bool,
	#[serde(rename = "expiresAt")]
	expires_at: Option<DateTime<Utc>>,
	choices: Vec<PackedPollChoices>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackedPollChoices {
	text: String,
	votes: i32,
	#[serde(rename = "isVoted")]
	is_voted: bool,
}
