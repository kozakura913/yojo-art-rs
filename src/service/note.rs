use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::{
	DBConnection, DBConnectionRef, DataBase, MisskeyConfig, ServerError,
	models::{
		self,
		common::SearchableTypes,
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
	pub async fn pack_detail(
		&self,
		con: &mut DBConnection<'_>,
		note: MiNote,
		me_id: &String,
		user_cache: &mut HashMap<String, MiUser>,
		note_cache: &mut HashMap<String, PackedNote>,
		note_hint: &HashMap<String, MiNote>,
	) -> Result<PackedNote, ServerError> {
		let reply = match note.reply_id.as_ref() {
			Some(reply_id) => {
				let reply = match note_cache.get(reply_id) {
					Some(note) => note.clone(),
					None => {
						let note = match note_hint.get(reply_id) {
							Some(note) => note.clone(),
							None => MiNote::load_by_id(con, reply_id).await?,
						};
						let packed = self.pack(con, note, me_id, user_cache).await?;
						note_cache.insert(packed.id.clone(), packed.clone());
						packed
					}
				};
				Some(Box::new(reply))
			}
			None => None,
		};
		let renote = match note.renote_id.as_ref() {
			Some(renote_id) => {
				let renote = match note_cache.get(renote_id) {
					Some(note) => note.clone(),
					None => {
						let note = match note_hint.get(renote_id) {
							Some(note) => note.clone(),
							None => MiNote::load_by_id(con, renote_id).await?,
						};
						let packed = self.pack(con, note, me_id, user_cache).await?;
						note_cache.insert(packed.id.clone(), packed.clone());
						packed
					}
				};
				Some(Box::new(renote))
			}
			None => None,
		};
		let mut packed_note = self.pack(con, note, me_id, user_cache).await?;
		packed_note.renote = renote;
		packed_note.reply = reply;
		Ok(packed_note)
	}
	/* renoteとreplyを処理しない */
	pub async fn pack(
		&self,
		con: &mut DBConnection<'_>,
		note: MiNote,
		me_id: &String,
		user_cache: &mut HashMap<String, MiUser>,
	) -> Result<PackedNote, ServerError> {
		let user = match user_cache.get(&note.user_id) {
			Some(u) => u,
			None => {
				let u = MiUser::load_by_id(con, &note.user_id).await?;
				user_cache.insert(note.user_id.clone(), u);
				user_cache.get(&note.user_id).ok_or("no user")?
			}
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
		let files = {
			let files: Vec<MiDriveFile> = {
				use crate::models::drive_file::drive_file::dsl::drive_file;
				use crate::models::drive_file::drive_file::dsl::*;
				use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
				use diesel_async::RunQueryDsl;
				drive_file
					.filter(id.eq_any(&note.file_ids))
					.select(MiDriveFile::as_select())
					.load(con)
					.await
			}?;
			let mut packed_files = Vec::new();
			for f in files.iter() {
				let is_my_file = f.user_id.as_ref() == Some(me_id);
				let packed = self
					.drive_service
					.pack(con, f, is_my_file, false, false, None, Some(&user))
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
					.populate_emojis(con, emojis, Some(host.clone()))
					.await,
			)
		} else {
			None
		};
		let reaction_emojis = self
			.emoji_service
			.populate_emojis(con, reaction_emoji_names, user.host.clone())
			.await;
		let event = if note.has_event {
			self.populate_event(&mut con.into(), &note.id).await
		} else {
			None
		};
		let poll = if note.has_poll {
			self.populate_poll(&mut con.into(), &note.id, Some(me_id))
				.await
		} else {
			None
		};
		let user = self.user_service.pack_lite(user.clone()).await?;
		let created_at = self.id_service.parse(&note.id).ok_or("parse id")?;
		let my_reaction = if reaction_count < 1 {
			None
		} else if reaction_count as usize <= note.reaction_and_user_pair_cache.len() {
			let mut reactions: Vec<String> = Vec::new();
			for cache in note.reaction_and_user_pair_cache.iter() {
				if !cache.starts_with(me_id) {
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
						.filter(userId.eq(&me_id))
						.select(MiNoteReaction::as_select())
						.load(con)
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
		con: &mut DBConnectionRef<'_, '_>,
		note_id: &String,
		me_id: Option<&String>,
	) -> Option<PackedPoll> {
		let mi_poll: MiPoll = {
			use crate::models::poll::poll::dsl::noteId;
			use crate::models::poll::poll::dsl::poll;
			use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
			use diesel_async::RunQueryDsl;
			let query = poll.filter(noteId.eq(note_id)).select(MiPoll::as_select());
			match con {
				DBConnectionRef::Borrowed(con) => query.first(*con).await,
				DBConnectionRef::Mutex(m) => {
					let mut con = m.lock().await;
					query.first(&mut con).await
				}
			}
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
				let query = poll_vote
					.filter(noteId.eq(note_id))
					.filter(userId.eq(me_id))
					.select(MiPollVote::as_select());
				match con {
					DBConnectionRef::Borrowed(con) => query.load(con).await,
					DBConnectionRef::Mutex(m) => {
						let mut con = m.lock().await;
						query.load(&mut con).await
					}
				}
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
	pub async fn populate_event(
		&self,
		con: &mut DBConnectionRef<'_, '_>,
		note_id: &String,
	) -> Option<PackedEvent> {
		let event: MiEvent = {
			use crate::models::event::event::dsl::event;
			use crate::models::event::event::dsl::noteId;
			use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
			use diesel_async::RunQueryDsl;
			let query = event
				.filter(noteId.eq(note_id))
				.select(MiEvent::as_select());
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
	searchable_by: Option<SearchableTypes>,
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
