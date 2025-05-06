use chrono::NaiveDateTime;
use diesel::{
	FromSqlRow, Selectable,
	deserialize::FromSql,
	expression::AsExpression,
	serialize::{IsNull, ToSql},
	sql_types::{Jsonb, VarChar},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum_macros::{Display, EnumString};

diesel::table! {
	#[sql_name = "note"]
	note (id) {
		id -> VarChar,
		updatedAt -> Nullable<Timestamp>,
		updatedAtHistory -> Nullable<Array<Timestamp>>,
		noteEditHistory -> Array<VarChar>,
		replyId -> Nullable<VarChar>,
		renoteId -> Nullable<VarChar>,
		threadId -> Nullable<VarChar>,
		hasEvent -> Bool,
		text -> Text,
		name -> Nullable<VarChar>,
		cw -> Nullable<VarChar>,
		userId -> Nullable<VarChar>,
		localOnly -> Bool,
		reactionAcceptance -> Nullable<VarChar>,
		disableRightClick -> Bool,
		renoteCount -> SmallInt,
		repliesCount -> SmallInt,
		clippedCount -> SmallInt,
		reactions -> Jsonb,
		visibility -> VarChar,
		searchableBy -> Nullable<VarChar>,
		uri -> Nullable<VarChar>,
		url -> Nullable<VarChar>,
		fileIds -> Array<VarChar>,
		attachedFileTypes -> Array<VarChar>,
		visibleUserIds -> Array<VarChar>,
		mentions -> Array<VarChar>,
		mentionedRemoteUsers -> Text,
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
	pub text: String,
	pub name: Option<String>,
	pub cw: Option<String>,
	#[diesel(column_name = "userId")]
	pub user_id: Option<String>,
	#[diesel(column_name = "localOnly")]
	pub local_only: bool,
	#[diesel(column_name = "reactionAcceptance")]
	pub reaction_acceptance: NoteReactionAcceptances,
	#[diesel(column_name = "disableRightClick")]
	pub disable_right_click: bool,
	#[diesel(column_name = "renoteCount")]
	pub renote_count: i16,
	#[diesel(column_name = "repliesCount")]
	pub replies_count: i16,
	#[diesel(column_name = "clippedCount")]
	pub clipped_count: i16,
	pub reactions: MiReactions,
	pub visibility: NoteVisibilities,
	#[diesel(column_name = "searchableBy")]
	pub searchable_by: SearchableTypes,
	/** The URI of a note. it will be null when the note is local. */
	pub uri:Option<String>,
	/** The human readable url of a note. it will be null when the note is local. */
	pub url:Option<String>,
	#[diesel(column_name = "fileIds")]
	pub file_ids: Vec<String>,
	#[diesel(column_name = "attachedFileTypes")]
	pub attached_file_types: Vec<String>,
	#[diesel(column_name = "visibleUserIds")]
	pub visible_user_ids: Vec<String>,
	pub mentions: Vec<String>,
	#[diesel(column_name = "mentionedRemoteUsers")]
	pub mentioned_remote_users:String,
}
#[derive(Copy, Clone, EnumString, Display, Default, Debug, FromSqlRow, AsExpression)]
#[diesel(sql_type = Nullable<VarChar>)]
pub enum NoteReactionAcceptances {
	#[strum(serialize = "likeOnly")]
	LikeOnly,
	#[strum(serialize = "likeOnlyForRemote")]
	LikeOnlyForRemote,
	#[strum(serialize = "nonSensitiveOnly")]
	NonSensitiveOnly,
	#[strum(serialize = "nonSensitiveOnlyForLocalLikeOnlyForRemote")]
	NonSensitiveOnlyForLocalLikeOnlyForRemote,
	#[strum(serialize = "")]
	#[default]
	None,
}
impl ToSql<diesel::sql_types::Nullable<VarChar>, diesel::pg::Pg> for NoteReactionAcceptances
where
	String: ToSql<VarChar, diesel::pg::Pg>,
{
	fn to_sql<'b>(
		&'b self,
		out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
	) -> diesel::serialize::Result {
		if let Self::None = &self {
			return diesel::serialize::Result::Ok(IsNull::Yes);
		}
		<String as ToSql<VarChar, diesel::pg::Pg>>::to_sql(&self.to_string(), &mut out.reborrow())
	}
}
impl<DB: diesel::backend::Backend> FromSql<diesel::sql_types::Nullable<VarChar>, DB> for NoteReactionAcceptances
where
	String: FromSql<VarChar, DB>,
{
	fn from_sql(bytes: DB::RawValue<'_>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let v =
			<Option<String> as FromSql<diesel::sql_types::Nullable<VarChar>, DB>>::from_sql(bytes)?;
		if let Some(v) = v {
			use std::str::FromStr;
			Self::from_str(&v).or_else(|_| Ok(Self::None))
		} else {
			Ok(Self::None)
		}
	}
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Jsonb)]
pub struct MiReactions(pub HashMap<String, String>);
impl ToSql<Jsonb, diesel::pg::Pg> for MiReactions
where
	serde_json::Value: ToSql<Jsonb, diesel::pg::Pg>,
{
	fn to_sql<'b>(
		&'b self,
		out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
	) -> diesel::serialize::Result {
		<serde_json::Value as ToSql<Jsonb, diesel::pg::Pg>>::to_sql(
			&(serde_json::to_value(&self).map_err(|e| Box::new(e))?),
			&mut out.reborrow(),
		)
	}
}
impl<DB: diesel::backend::Backend> FromSql<Jsonb, DB> for MiReactions
where
	serde_json::Value: FromSql<Jsonb, DB>,
{
	fn from_sql(bytes: DB::RawValue<'_>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let v = <serde_json::Value as FromSql<Jsonb, DB>>::from_sql(bytes)?;
		Ok(serde_json::from_str::<Self>(&v.to_string()).map_err(|e| Box::new(e))?)
	}
}
#[derive(Copy, Clone, EnumString, Display, Default, Debug, FromSqlRow, AsExpression)]
#[diesel(sql_type = VarChar)]
pub enum NoteVisibilities {
	#[default]
	#[strum(serialize = "public")]
	/** 公開 */
	Public,
	#[strum(serialize = "home")]
	/** ホームタイムライン(ユーザーページのタイムライン含む)のみに流す */
	Home,
	#[strum(serialize = "followers")]
	/** フォロワーのみ */
	Followers,
	#[strum(serialize = "specified")]
	/** visibleUserIds で指定したユーザーのみ */
	Specified,
}
impl ToSql<VarChar, diesel::pg::Pg> for NoteVisibilities
where
	String: ToSql<VarChar, diesel::pg::Pg>,
{
	fn to_sql<'b>(
		&'b self,
		out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
	) -> diesel::serialize::Result {
		<String as ToSql<VarChar, diesel::pg::Pg>>::to_sql(&self.to_string(), &mut out.reborrow())
	}
}
impl<DB: diesel::backend::Backend> FromSql<VarChar, DB> for NoteVisibilities
where
	String: FromSql<VarChar, DB>,
{
	fn from_sql(bytes: DB::RawValue<'_>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let v =<String as FromSql<VarChar, DB>>::from_sql(bytes)?;
		use std::str::FromStr;
		Self::from_str(&v).or_else(|_| Ok(Self::Home))
	}
}


#[derive(Copy, Clone, EnumString, Display, Default, Debug, FromSqlRow, AsExpression)]
#[diesel(sql_type = Nullable<VarChar>)]
pub enum SearchableTypes {
	#[strum(serialize = "public")]
	/** だれでも */
	Public,
	#[strum(serialize = "followers")]
	/** フォロワーのみ */
	Followers,
	#[strum(serialize = "reacted")]
	/** 返信かリアクションしたユーザーのみ */
	Reacted,
	#[strum(serialize = "")]
	#[default]
	/** ユーザーのsearchableByを見る */
	None,
}
impl ToSql<diesel::sql_types::Nullable<VarChar>, diesel::pg::Pg> for SearchableTypes
where
	String: ToSql<VarChar, diesel::pg::Pg>,
{
	fn to_sql<'b>(
		&'b self,
		out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
	) -> diesel::serialize::Result {
		if let Self::None = &self {
			return diesel::serialize::Result::Ok(IsNull::Yes);
		}
		<String as ToSql<VarChar, diesel::pg::Pg>>::to_sql(&self.to_string(), &mut out.reborrow())
	}
}
impl<DB: diesel::backend::Backend> FromSql<diesel::sql_types::Nullable<VarChar>, DB> for SearchableTypes
where
	String: FromSql<VarChar, DB>,
{
	fn from_sql(bytes: DB::RawValue<'_>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let v =
			<Option<String> as FromSql<diesel::sql_types::Nullable<VarChar>, DB>>::from_sql(bytes)?;
		if let Some(v) = v {
			use std::str::FromStr;
			Self::from_str(&v).or_else(|_| Ok(Self::None))
		} else {
			Ok(Self::None)
		}
	}
}
/*
	@Column('varchar', {
		length: 1024, array: true, default: '{}',
	})
	public reactionAndUserPairCache: string[];

	@Column('varchar', {
		length: 128, array: true, default: '{}',
	})
	public emojis: string[];

	@Index('IDX_NOTE_TAGS', { synchronize: false })
	@Column('varchar', {
		length: 128, array: true, default: '{}',
	})
	public tags: string[];

	@Column('boolean', {
		default: false,
	})
	public hasPoll: boolean;

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
