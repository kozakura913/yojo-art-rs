use diesel::{deserialize::{FromSql, FromSqlRow}, expression::AsExpression, serialize::ToSql, sql_types::VarChar, ExpressionMethods, QueryDsl, Selectable, SelectableHelper};
use diesel_async::RunQueryDsl;
use strum_macros::{Display, EnumString};
use crate::DBConnection;

diesel::table! {
	#[sql_name = "user_profile"]
	user_profile (userId) {
		userId -> VarChar,
		alwaysMarkNsfw -> Bool,
		autoSensitive -> Bool,
		followingVisibility -> VarChar,
		followersVisibility -> VarChar,
	}
}
#[derive(PartialEq, Eq,Debug,Clone,diesel::Insertable,diesel::Queryable,Selectable,diesel::QueryableByName)]
#[diesel(table_name = user_profile)]
pub struct MiUserProfile{
	#[diesel(column_name = "userId")]
	pub user_id:String,
	#[diesel(column_name = "alwaysMarkNsfw")]
	pub always_mark_nsfw: bool,
	#[diesel(column_name = "autoSensitive")]
	pub auto_sensitive: bool,
	#[diesel(column_name = "followingVisibility")]
	pub following_visibility:Visibility,
	#[diesel(column_name = "followersVisibility")]
	pub followers_visibility:Visibility,
}
#[derive(PartialEq,Eq,Copy,Clone,EnumString,Display,Default,Debug,FromSqlRow, AsExpression)]
#[diesel(sql_type = VarChar)]
pub enum Visibility{
	#[default]
	#[strum(serialize = "public")]
	Public,
	#[strum(serialize = "followers")]
	Followers,
	#[strum(serialize = "private")]
	Private
}
impl ToSql<VarChar, diesel::pg::Pg> for Visibility where String: ToSql<VarChar, diesel::pg::Pg>{
	fn to_sql<'b>(&'b self,out:&mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result{
		<String as ToSql<VarChar, diesel::pg::Pg>>::to_sql(&self.to_string(), &mut out.reborrow())
	}
}
impl<DB: diesel::backend::Backend> FromSql<VarChar, DB> for Visibility where String: FromSql<VarChar, DB>{
	fn from_sql(bytes: DB::RawValue<'_>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let v=<String as FromSql<VarChar, DB>>::from_sql(bytes)?;
		use std::str::FromStr;
		Self::from_str(&v).or_else(|_|Ok(Self::Private))
	}
}

impl MiUserProfile{
	pub async fn load_by_user(con:&mut DBConnection<'_>,user_id:&str)->Option<Self>{
		let res:MiUserProfile={
			use self::user_profile::dsl::user_profile;
			use self::user_profile::dsl::*;
			user_profile.filter(userId.eq(user_id)).select(MiUserProfile::as_select()).first(con).await.map_err(|e|{
				eprintln!("{:?}",e);
			})
		}.ok()?;
		Some(res)
	}
}
/*

// TODO: このテーブルで管理している情報すべてレジストリで管理するようにしても良いかも
//       ただ、「emailVerified が true なユーザーを find する」のようなクエリは書けなくなるからウーン
@Entity('user_profile')
export class MiUserProfile {
	@PrimaryColumn(id())
	public userId: MiUser['id'];

	@OneToOne(type => MiUser, {
		onDelete: 'CASCADE',
	})
	@JoinColumn()
	public user: MiUser | null;

	@Column('varchar', {
		length: 128, nullable: true,
		comment: 'The location of the User.',
	})
	public location: string | null;

	@Index()
	@Column('char', {
		length: 10, nullable: true,
		comment: 'The birthday (YYYY-MM-DD) of the User.',
	})
	public birthday: string | null;

	@Column('varchar', {
		length: 2048, nullable: true,
		comment: 'The description (bio) of the User.',
	})
	public description: string | null;

	@Column('jsonb', {
		default: [],
	})
	public mutualLinkSections: {
		name: string | null;
		mutualLinks: {
			id: string;
			fileId: MiDriveFile['id'];
			description: string | null;
			imgSrc: string;
			url: string;
		}[];
	}[] | [];

	@Column('jsonb', {
		default: [],
	})
	public fields: {
		name: string;
		value: string;
	}[];

	@Column('varchar', {
		array: true,
		default: '{}',
	})
	public verifiedLinks: string[];

	@Column('varchar', {
		length: 32, nullable: true,
	})
	public lang: string | null;

	@Column('varchar', {
		length: 512, nullable: true,
		comment: 'Remote URL of the user.',
	})
	public url: string | null;

	@Column('varchar', {
		length: 128, nullable: true,
		comment: 'The email address of the User.',
	})
	public email: string | null;

	@Column('varchar', {
		length: 128, nullable: true,
	})
	public emailVerifyCode: string | null;

	@Column('boolean', {
		default: false,
	})
	public emailVerified: boolean;

	@Column('jsonb', {
		default: ['follow', 'receiveFollowRequest', 'groupInvited'],
	})
	public emailNotificationTypes: string[];

	@Column('boolean', {
		default: true,
	})
	public publicReactions: boolean;

	@Column('enum', {
		enum: followingVisibilities,
		default: 'public',
	})
	public followingVisibility: typeof followingVisibilities[number];

	@Column('enum', {
		enum: followersVisibilities,
		default: 'public',
	})
	public followersVisibility: typeof followersVisibilities[number];

	@Column('varchar', {
		length: 128, nullable: true,
	})
	public twoFactorTempSecret: string | null;

	@Column('varchar', {
		length: 128, nullable: true,
	})
	public twoFactorSecret: string | null;

	@Column('varchar', {
		nullable: true, array: true,
	})
	public twoFactorBackupSecret: string[] | null;

	@Column('boolean', {
		default: false,
	})
	public twoFactorEnabled: boolean;

	@Column('boolean', {
		default: false,
	})
	public securityKeysAvailable: boolean;

	@Column('boolean', {
		default: false,
	})
	public usePasswordLessLogin: boolean;

	@Column('varchar', {
		length: 128, nullable: true,
		comment: 'The password hash of the User. It will be null if the origin of the user is local.',
	})
	public password: string | null;

	@Column('varchar', {
		length: 8192, default: '',
	})
	public moderationNote: string | null;

	// TODO: そのうち消す
	@Column('jsonb', {
		default: {},
		comment: 'The client-specific data of the User.',
	})
	public clientData: Record<string, any>;

	// TODO: そのうち消す
	@Column('jsonb', {
		default: {},
		comment: 'The room data of the User.',
	})
	public room: Record<string, any>;

	@Column('boolean', {
		default: false,
	})
	public autoAcceptFollowed: boolean;

	@Column('boolean', {
		default: false,
		comment: 'Whether reject index by crawler.',
	})
	public noCrawle: boolean;

	@Column('boolean', {
		default: true,
	})
	public preventAiLearning: boolean;

	@Column('boolean', {
		default: false,
	})
	public alwaysMarkNsfw: boolean;

	@Column('boolean', {
		default: false,
	})
	public autoSensitive: boolean;

	@Column('boolean', {
		default: false,
	})
	public carefulBot: boolean;

	@Column('boolean', {
		default: true,
	})
	public injectFeaturedNote: boolean;

	@Column('boolean', {
		default: true,
	})
	public receiveAnnouncementEmail: boolean;

	@Column({
		...id(),
		nullable: true,
	})
	public pinnedPageId: MiPage['id'] | null;

	@OneToOne(type => MiPage, {
		onDelete: 'SET NULL',
	})
	@JoinColumn()
	public pinnedPage: MiPage | null;

	@Index()
	@Column('boolean', {
		default: false, select: false,
	})
	public enableWordMute: boolean;

	@Column('jsonb', {
		default: [],
	})
	public mutedWords: (string[] | string)[];

	@Column('jsonb', {
		default: [],
	})
	public hardMutedWords: (string[] | string)[];

	@Column('jsonb', {
		default: [],
		comment: 'List of instances muted by the user.',
	})
	public mutedInstances: string[];

	@Column('jsonb', {
		default: {},
	})
	public notificationRecieveConfig: {
		[notificationType in typeof notificationTypes[number]]?: {
			type: 'all';
		} | {
			type: 'never';
		} | {
			type: 'following';
		} | {
			type: 'follower';
		} | {
			type: 'mutualFollow';
		} | {
			type: 'followingOrFollower';
		} | {
			type: 'list';
			userListId: MiUserList['id'];
		};
	};

	@Column('varchar', {
		length: 32, array: true, default: '{}',
	})
	public loggedInDates: string[];

	@Column('jsonb', {
		default: [],
	})
	public achievements: {
		name: string;
		unlockedAt: number;
	}[];

	//#region Denormalized fields
	@Index()
	@Column('varchar', {
		length: 128, nullable: true,
		comment: '[Denormalized]',
	})
	public userHost: string | null;
	//#endregion

	constructor(data: Partial<MiUserProfile>) {
		if (data == null) return;

		for (const [k, v] of Object.entries(data)) {
			(this as any)[k] = v;
		}
	}
}


*/