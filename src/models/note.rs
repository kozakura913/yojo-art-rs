use chrono::NaiveDateTime;
use diesel::{
	FromSqlRow, Selectable,
	deserialize::FromSql,
	expression::AsExpression,
	serialize::ToSql,
	sql_types::{Jsonb, VarChar},
};
use serde::{Deserialize, Serialize};

use std::collections::{HashMap, HashSet};
use strum_macros::{Display, EnumString};
use yojo_art_utils::{PgEnum, PgJson, PgString};

use crate::{DBConnection, models::common::NoteSearchableBy};
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;

diesel::table! {
	#[sql_name = "note"]
	note (id) {
		id -> VarChar,
		updatedAt -> Nullable<Timestamp>,
		updatedAtHistory -> Nullable<Array<Timestamp>>,
		deleteAt -> Nullable<Timestamp>,
		noteEditHistory -> Array<VarChar>,
		replyId -> Nullable<VarChar>,
		renoteId -> Nullable<VarChar>,
		threadId -> Nullable<VarChar>,
		hasEvent -> Bool,
		text -> Nullable<Text>,
		name -> Nullable<VarChar>,
		cw -> Nullable<VarChar>,
		userId -> VarChar,
		userHost -> Nullable<VarChar>,
		localOnly -> Bool,
		reactionAcceptance -> Nullable<VarChar>,
		disableRightClick -> Bool,
		renoteCount -> SmallInt,
		repliesCount -> SmallInt,
		clippedCount -> SmallInt,
		reactions -> Jsonb,
		visibility -> crate::models::note::NoteVisibilityType,
		searchableBy -> Nullable<crate::models::common::NoteSearchableType>,
		uri -> Nullable<VarChar>,
		url -> Nullable<VarChar>,
		fileIds -> Array<VarChar>,
		attachedFileTypes -> Array<VarChar>,
		visibleUserIds -> Array<VarChar>,
		mentions -> Array<VarChar>,
		mentionedRemoteUsers -> Text,
		emojis -> Array<VarChar>,
		tags -> Array<VarChar>,
		hasPoll -> Bool,
		reactionAndUserPairCache -> Array<VarChar>,
	}
}
#[derive(
	Debug, Clone, diesel::Insertable, diesel::Queryable, Selectable, diesel::QueryableByName,
)]
#[diesel(table_name = note)]
pub struct MiNote {
	pub id: String,
	#[diesel(column_name = "updatedAt")]
	pub updated_at: Option<NaiveDateTime>,
	#[diesel(column_name = "updatedAtHistory")]
	pub updated_at_history: Option<Vec<NaiveDateTime>>,
	#[diesel(column_name = "deleteAt")]
	pub delete_at: Option<NaiveDateTime>,
	#[diesel(column_name = "noteEditHistory")]
	pub note_edit_history: Vec<String>,
	#[diesel(column_name = "replyId")]
	pub reply_id: Option<String>,
	#[diesel(column_name = "renoteId")]
	pub renote_id: Option<String>,
	#[diesel(column_name = "threadId")]
	pub thread_id: Option<String>,
	#[diesel(column_name = "hasEvent")]
	pub has_event: bool,
	pub text: Option<String>,
	pub name: Option<String>,
	pub cw: Option<String>,
	#[diesel(column_name = "userId")]
	pub user_id: String,
	#[diesel(column_name = "userHost")]
	pub user_host: Option<String>,
	#[diesel(column_name = "localOnly")]
	pub local_only: bool,
	#[diesel(column_name = "reactionAcceptance")]
	pub reaction_acceptance: Option<NoteReactionAcceptances>,
	#[diesel(column_name = "disableRightClick")]
	pub disable_right_click: bool,
	#[diesel(column_name = "renoteCount")]
	pub renote_count: i16,
	#[diesel(column_name = "repliesCount")]
	pub replies_count: i16,
	#[diesel(column_name = "clippedCount")]
	pub clipped_count: i16,
	pub reactions: MiReactions,
	pub visibility: NoteVisibility,
	#[diesel(column_name = "searchableBy")]
	/** NoneでユーザーのsearchableByを見る */
	pub searchable_by: Option<NoteSearchableBy>,
	/** The URI of a note. it will be null when the note is local. */
	pub uri: Option<String>,
	/** The human readable url of a note. it will be null when the note is local. */
	pub url: Option<String>,
	#[diesel(column_name = "fileIds")]
	pub file_ids: Vec<String>,
	#[diesel(column_name = "attachedFileTypes")]
	pub attached_file_types: Vec<String>,
	#[diesel(column_name = "visibleUserIds")]
	pub visible_user_ids: Vec<String>,
	pub mentions: Vec<String>,
	#[diesel(column_name = "mentionedRemoteUsers")]
	pub mentioned_remote_users: String,
	pub emojis: Vec<String>,
	pub tags: Vec<String>,
	#[diesel(column_name = "hasPoll")]
	pub has_poll: bool,
	#[diesel(column_name = "reactionAndUserPairCache")]
	pub reaction_and_user_pair_cache: Vec<String>,
}
impl MiNote {
	pub fn is_visible(&self, user_id: &String, followings: Option<&HashSet<String>>) -> bool {
		if &self.user_id == user_id {
			return true;
		}
		match self.visibility {
			NoteVisibility::Public | NoteVisibility::Home => true,
			NoteVisibility::Followers => {
				if let Some(set) = followings {
					set.contains(user_id)
				} else {
					false
				}
			}
			NoteVisibility::Specified => self.visible_user_ids.contains(user_id),
		}
	}
	pub fn is_renote(&self) -> bool {
		self.renote_id.is_some()
	}
	pub fn is_quote(&self) -> bool {
		self.text.is_some()
			|| self.cw.is_some()
			|| self.reply_id.is_some()
			|| self.has_poll
			|| !self.file_ids.is_empty()
	}
}
#[derive(
	Copy,
	Clone,
	EnumString,
	Display,
	Debug,
	FromSqlRow,
	AsExpression,
	Serialize,
	Deserialize,
	PgString,
)]
#[diesel(sql_type = VarChar)]
pub enum NoteReactionAcceptances {
	#[strum(serialize = "likeOnly")]
	#[serde(rename = "likeOnly")]
	LikeOnly,
	#[strum(serialize = "likeOnlyForRemote")]
	#[serde(rename = "likeOnlyForRemote")]
	LikeOnlyForRemote,
	#[strum(serialize = "nonSensitiveOnly")]
	#[serde(rename = "nonSensitiveOnly")]
	NonSensitiveOnly,
	#[strum(serialize = "nonSensitiveOnlyForLocalLikeOnlyForRemote")]
	#[serde(rename = "nonSensitiveOnlyForLocalLikeOnlyForRemote")]
	NonSensitiveOnlyForLocalLikeOnlyForRemote,
}
#[derive(Clone, Default, Debug, Serialize, Deserialize, FromSqlRow, AsExpression, PgJson)]
#[diesel(sql_type = Jsonb)]
pub struct MiReactions(pub HashMap<String, i32>);
#[derive(
	Copy,
	Clone,
	PartialEq,
	Eq,
	EnumString,
	Display,
	Default,
	Debug,
	FromSqlRow,
	AsExpression,
	Serialize,
	Deserialize,
	PgEnum,
)]
#[diesel(sql_type = NoteVisibilityType)]
#[pg_type(sql_type = "NoteVisibilityType")]
pub enum NoteVisibility {
	#[default]
	#[strum(serialize = "public")]
	#[serde(rename = "public")]
	/** 公開 */
	Public,
	#[strum(serialize = "home")]
	#[serde(rename = "home")]
	/** ホームタイムライン(ユーザーページのタイムライン含む)のみに流す */
	Home,
	#[strum(serialize = "followers")]
	#[serde(rename = "followers")]
	/** フォロワーのみ */
	Followers,
	#[strum(serialize = "specified")]
	#[serde(rename = "specified")]
	/** visibleUserIds で指定したユーザーのみ */
	Specified,
}
#[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
#[diesel(postgres_type(name = "note_visibility_enum"))]
pub struct NoteVisibilityType;
impl MiNote {
	pub async fn load_by_id(
		con: &mut DBConnection<'_>,
		note_id: &str,
	) -> Result<Self, diesel::result::Error> {
		use self::note::dsl::note;
		use self::note::dsl::*;
		note.filter(id.eq(note_id))
			.select(Self::as_select())
			.first(con)
			.await
	}
}
/*
	@Column('varchar', {
		length: 1024, array: true, default: '{}',
	})
	public reactionAndUserPairCache: string[];

	@Index()
	@Column({
		...id(),
		nullable: true,
		comment: 'The ID of source channel.',
	})
	public channelId: MiChannel['id'] | null;

	@ManyToOne(type => MiChannel, {
		onDelete: 'CASCADE',
	})
	@JoinColumn()
	public channel: MiChannel | null;

	//#region Denormalized fields
	@Index()
	@Column('varchar', {
		length: 128, nullable: true,
		comment: '[Denormalized]',
	})
	public userHost: string | null;

	@Column({
		...id(),
		nullable: true,
		comment: '[Denormalized]',
	})
	public replyUserId: MiUser['id'] | null;

	@Column('varchar', {
		length: 128, nullable: true,
		comment: '[Denormalized]',
	})
	public replyUserHost: string | null;

	@Column({
		...id(),
		nullable: true,
		comment: '[Denormalized]',
	})
	public renoteUserId: MiUser['id'] | null;

	@Column('varchar', {
		length: 128, nullable: true,
		comment: '[Denormalized]',
	})
	public renoteUserHost: string | null;

	@Column('timestamp with time zone', {
		nullable: true,
	})
	public deleteAt: Date | null;
	//#endregion

	constructor(data: Partial<MiNote>) {
		if (data == null) return;

		for (const [k, v] of Object.entries(data)) {
			(this as any)[k] = v;
		}
	}
}

export type IMentionedRemoteUsers = {
	uri: string;
	url?: string;
	username: string;
	host: string;
}[];
*/
