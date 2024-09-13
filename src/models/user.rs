use chrono::NaiveDateTime;
use diesel::{ExpressionMethods, QueryDsl, Selectable, SelectableHelper};
use diesel_async::RunQueryDsl;

use crate::DBConnection;

diesel::table! {
	#[sql_name = "user"]
	user (id) {
		id -> VarChar,
		updatedAt -> Nullable<Timestamp>,
		lastFetchedAt -> Nullable<Timestamp>,
		lastActiveDate -> Nullable<Timestamp>,
		hideOnlineStatus -> Bool,
		username -> VarChar,
		usernameLower -> VarChar,
		name -> Nullable<VarChar>,
		followersCount -> Int4,
		followingCount -> Int4,
		token -> Nullable<VarChar>,
		host -> Nullable<VarChar>,
	}
}
#[derive(PartialEq,Eq,Debug,Clone,diesel::Insertable,diesel::Queryable,Selectable,diesel::QueryableByName)]
#[diesel(table_name = user)]
pub struct MiUser{
	pub id:String,
	#[diesel(column_name = "updatedAt")]
	pub updated_at:Option<NaiveDateTime>,
	#[diesel(column_name = "lastFetchedAt")]
	pub last_fetched_at:Option<NaiveDateTime>,
	#[diesel(column_name = "lastActiveDate")]
	pub last_active_date:Option<NaiveDateTime>,
	#[diesel(column_name = "hideOnlineStatus")]
	pub hide_online_status:bool,
	pub username:String,
	#[diesel(column_name = "usernameLower")]
	pub username_lower:String,
	#[diesel(column_name = "name")]
	pub display_name:Option<String>,
	#[diesel(column_name = "followersCount")]
	pub followers_count:i32,
	#[diesel(column_name = "followingCount")]
	pub following_count:i32,
	pub token:Option<String>,//リモートユーザーは持たない
	pub host:Option<String>,//ローカルユーザーは持たない
}
impl MiUser{
	pub async fn load_by_id(con:&mut DBConnection<'_>,user_id:&str)->Option<Self>{
		let res:MiUser={
			use self::user::dsl::user;
			use self::user::dsl::*;
			user.filter(id.eq(user_id)).select(MiUser::as_select()).first(con).await.map_err(|e|{
				eprintln!("{:?}",e);
			})
		}.ok()?;
		Some(res)
	}
	pub async fn load_by_token(con:&mut DBConnection<'_>,user_token:&str)->Option<Self>{
		let res:MiUser={
			use self::user::dsl::user;
			use self::user::dsl::*;
			user.filter(token.eq(user_token)).select(MiUser::as_select()).first(con).await.map_err(|e|{
				eprintln!("{:?}",e);
			})
		}.ok()?;
		Some(res)
	}
}
/*
	@Column('varchar', {
		length: 512,
		nullable: true,
		comment: 'The URI of the new account of the User',
	})
	public movedToUri: string | null;

	@Column('timestamp with time zone', {
		nullable: true,
		comment: 'When the user moved to another account',
	})
	public movedAt: Date | null;

	@Column('simple-array', {
		nullable: true,
		comment: 'URIs the user is known as too',
	})
	public alsoKnownAs: string[] | null;

	@Column('integer', {
		default: 0,
		comment: 'The count of notes.',
	})
	public notesCount: number;

	@Column({
		...id(),
		nullable: true,
		comment: 'The ID of avatar DriveFile.',
	})
	public avatarId: MiDriveFile['id'] | null;

	@OneToOne(type => MiDriveFile, {
		onDelete: 'SET NULL',
	})
	@JoinColumn()
	public avatar: MiDriveFile | null;

	@Column({
		...id(),
		nullable: true,
		comment: 'The ID of banner DriveFile.',
	})
	public bannerId: MiDriveFile['id'] | null;

	@OneToOne(type => MiDriveFile, {
		onDelete: 'SET NULL',
	})
	@JoinColumn()
	public banner: MiDriveFile | null;

	@Column('varchar', {
		length: 512, nullable: true,
	})
	public avatarUrl: string | null;

	@Column('varchar', {
		length: 512, nullable: true,
	})
	public bannerUrl: string | null;

	@Column('varchar', {
		length: 128, nullable: true,
	})
	public avatarBlurhash: string | null;

	@Column('varchar', {
		length: 128, nullable: true,
	})
	public bannerBlurhash: string | null;

	@Column('jsonb', {
		default: [],
	})
	public avatarDecorations: {
		id: string;
		angle?: number;
		flipH?: boolean;
		offsetX?: number;
		offsetY?: number;
		scale?: number;
		opacity?: number;
	}[];

	@Index()
	@Column('varchar', {
		length: 128, array: true, default: '{}',
	})
	public tags: string[];

	@Column('boolean', {
		default: false,
		comment: 'Whether the User is suspended.',
	})
	public isSuspended: boolean;

	@Column('boolean', {
		default: false,
		comment: 'Whether the User is locked.',
	})
	public isLocked: boolean;

	@Column('boolean', {
		default: false,
		comment: 'Whether the User is a bot.',
	})
	public isBot: boolean;

	@Column('boolean', {
		default: false,
		comment: 'Whether the User is a cat.',
	})
	public isCat: boolean;

	@Column('boolean', {
		default: false,
		comment: 'Whether the User is the root.',
	})
	public isRoot: boolean;

	@Index()
	@Column('boolean', {
		default: true,
		comment: 'Whether the User is explorable.',
	})
	public isExplorable: boolean;

	@Column('boolean', {
		default: false,
	})
	public isHibernated: boolean;

	// アカウントが削除されたかどうかのフラグだが、完全に削除される際は物理削除なので実質削除されるまでの「削除が進行しているかどうか」のフラグ
	@Column('boolean', {
		default: false,
		comment: 'Whether the User is deleted.',
	})
	public isDeleted: boolean;

	@Column('varchar', {
		length: 128, array: true, default: '{}',
	})
	public emojis: string[];

	@Column('varchar', {
		length: 512, nullable: true,
		comment: 'The inbox URL of the User. It will be null if the origin of the user is local.',
	})
	public inbox: string | null;

	@Column('varchar', {
		length: 512, nullable: true,
		comment: 'The sharedInbox URL of the User. It will be null if the origin of the user is local.',
	})
	public sharedInbox: string | null;

	@Column('varchar', {
		length: 512, nullable: true,
	})
	public outbox: string | null;

	@Column('varchar', {
		length: 512, nullable: true,
		comment: 'The featured URL of the User. It will be null if the origin of the user is local.',
	})
	public featured: string | null;

	@Index()
	@Column('varchar', {
		length: 512, nullable: true,
		comment: 'The URI of the User. It will be null if the origin of the user is local.',
	})
	public uri: string | null;

	@Column('varchar', {
		length: 512, nullable: true,
		comment: 'The URI of the user Follower Collection. It will be null if the origin of the user is local.',
	})
	public followersUri: string | null;

	@Index({ unique: true })
	@Column('char', {
		length: 16, nullable: true, unique: true,
		comment: 'The native access token of the User. It will be null if the origin of the user is local.',
	})
	public token: string | null;

	constructor(data: Partial<MiUser>) {
		if (data == null) return;

		for (const [k, v] of Object.entries(data)) {
			(this as any)[k] = v;
		}
	}
*/
