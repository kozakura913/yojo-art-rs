use diesel::{deserialize::{FromSql, FromSqlRow}, expression::AsExpression, serialize::ToSql, sql_types::VarChar, Selectable};

diesel::table! {
	meta (id) {
		id -> VarChar,
		name -> Nullable<VarChar>,
		shortName -> Nullable<VarChar>,
		description -> Nullable<VarChar>,
		maintainerName -> Nullable<VarChar>,
		maintainerEmail -> Nullable<VarChar>,
		disableRegistration -> Bool,
		langs -> Array<VarChar>,
		pinnedUsers -> Array<VarChar>,
		hiddenTags -> Array<VarChar>,
		blockedHosts -> Array<VarChar>,
		sensitiveWords -> Array<VarChar>,
		prohibitedWords -> Array<VarChar>,
		silencedHosts -> Array<VarChar>,
		mediaSilencedHosts -> Array<VarChar>,
		themeColor -> Nullable<VarChar>,
		mascotImageUrl -> Nullable<VarChar>,
		bannerUrl -> Nullable<VarChar>,
		backgroundImageUrl -> Nullable<VarChar>,
		logoImageUrl -> Nullable<VarChar>,
		iconUrl -> Nullable<VarChar>,
		app192IconUrl -> Nullable<VarChar>,
		app512IconUrl -> Nullable<VarChar>,
		serverErrorImageUrl -> Nullable<VarChar>,
		notFoundImageUrl -> Nullable<VarChar>,
		infoImageUrl -> Nullable<VarChar>,
		cacheRemoteFiles -> Bool,
		cacheRemoteSensitiveFiles -> Bool,
		proxyAccountId -> Nullable<VarChar>,
		emailRequiredForSignup -> Bool,
		sensitiveMediaDetection -> VarChar,
		sensitiveMediaDetectionSensitivity -> VarChar,
		enableSensitiveMediaDetectionForVideos -> Bool,
		enableIpLogging -> Bool,
		policies -> Jsonb,
		setSensitiveFlagAutomatically -> Bool,
		enableChartsForFederatedInstances -> Bool,
	}
}

#[derive(PartialEq, Eq,Debug,Clone,diesel::Insertable,diesel::Queryable,Selectable,diesel::QueryableByName)]
#[diesel(table_name = meta)]
pub struct MiMeta{
	pub id:String,
	pub name:Option<String>,
	#[diesel(column_name = "shortName")]
	pub short_name:Option<String>,
	pub description:Option<String>,
	/**
	 * メンテナの名前
	 */
	#[diesel(column_name = "maintainerName")]
	pub maintainer_name:Option<String>,
	/**
	 * メンテナの連絡先
	 */
	#[diesel(column_name = "maintainerEmail")]
	pub maintainer_email:Option<String>,
	#[diesel(column_name = "disableRegistration")]
	pub disable_registration:bool,
	pub langs:Vec<String>,
	#[diesel(column_name = "pinnedUsers")]
	pub pinned_users:Vec<String>,
	#[diesel(column_name = "hiddenTags")]
	pub hidden_tags:Vec<String>,
	#[diesel(column_name = "blockedHosts")]
	pub blocked_hosts:Vec<String>,
	#[diesel(column_name = "sensitiveWords")]
	pub sensitive_words:Vec<String>,
	#[diesel(column_name = "prohibitedWords")]
	pub prohibited_words:Vec<String>,
	#[diesel(column_name = "silencedHosts")]
	pub silenced_wosts:Vec<String>,
	#[diesel(column_name = "mediaSilencedHosts")]
	pub media_silenced_hosts:Vec<String>,
	#[diesel(column_name = "themeColor")]
	pub theme_color:Option<String>,
	#[diesel(column_name = "mascotImageUrl")]
	pub mascot_image_url:Option<String>,
	#[diesel(column_name = "bannerUrl")]
	pub banner_url:Option<String>,
	#[diesel(column_name = "backgroundImageUrl")]
	pub background_image_url:Option<String>,
	#[diesel(column_name = "logoImageUrl")]
	pub logo_image_url:Option<String>,
	#[diesel(column_name = "iconUrl")]
	pub icon_url:Option<String>,
	#[diesel(column_name = "app192IconUrl")]
	pub app192_icon_url:Option<String>,
	#[diesel(column_name = "app512IconUrl")]
	pub app512_icon_url:Option<String>,
	#[diesel(column_name = "serverErrorImageUrl")]
	pub server_error_image_url:Option<String>,
	#[diesel(column_name = "notFoundImageUrl")]
	pub not_found_image_url:Option<String>,
	#[diesel(column_name = "infoImageUrl")]
	pub info_image_url:Option<String>,
	#[diesel(column_name = "cacheRemoteFiles")]
	pub cache_remote_files:bool,
	#[diesel(column_name = "cacheRemoteSensitiveFiles")]
	pub cache_remote_sensitive_files:bool,
	#[diesel(column_name = "proxyAccountId")]
	pub proxy_account_id:Option<String>,
	#[diesel(column_name = "emailRequiredForSignup")]
	pub email_required_for_signup:bool,
	#[diesel(column_name = "sensitiveMediaDetection")]
	pub sensitive_media_detection:SensitiveMediaDetection,
	#[diesel(column_name = "sensitiveMediaDetectionSensitivity")]
	pub sensitive_media_detection_sensitivity:SensitiveMediaDetectionSensitivity,
	#[diesel(column_name = "enableSensitiveMediaDetectionForVideos")]
	pub enable_sensitive_media_detection_for_videos:bool,
	#[diesel(column_name = "enableIpLogging")]
	pub enable_ip_logging:bool,
	pub policies:serde_json::Value,
	#[diesel(column_name = "setSensitiveFlagAutomatically")]
	pub set_sensitive_flag_automatically:bool,
	#[diesel(column_name = "enableChartsForFederatedInstances")]
	pub enable_charts_for_federated_instances:bool,
	
}

#[derive(PartialEq, Eq,Copy,Clone,strum_macros::EnumString,strum_macros::Display,Default,Debug,FromSqlRow, AsExpression)]
#[diesel(sql_type = VarChar)]
pub enum SensitiveMediaDetection{
	#[default]
	#[strum(serialize = "none")]
	None,
	#[strum(serialize = "all")]
	All,
	#[strum(serialize = "local")]
	Local,
	#[strum(serialize = "remote")]
	Remote,
}
impl ToSql<VarChar, diesel::pg::Pg> for SensitiveMediaDetection where String: ToSql<VarChar, diesel::pg::Pg>{
	fn to_sql<'b>(&'b self,out:&mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result{
		<String as ToSql<VarChar, diesel::pg::Pg>>::to_sql(&self.to_string(), &mut out.reborrow())
	}
}
impl<DB: diesel::backend::Backend> FromSql<VarChar, DB> for SensitiveMediaDetection where String: FromSql<VarChar, DB>{
	fn from_sql(bytes: DB::RawValue<'_>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let v=<String as FromSql<VarChar, DB>>::from_sql(bytes)?;
		use std::str::FromStr;
		Self::from_str(&v).or_else(|_|Ok(Self::default()))
	}
}
#[derive(PartialEq, Eq,Copy,Clone,strum_macros::EnumString,strum_macros::Display,Default,Debug,FromSqlRow, AsExpression)]
#[diesel(sql_type = VarChar)]
pub enum SensitiveMediaDetectionSensitivity{
	#[strum(serialize = "veryLow")]
	VeryLow,
	#[strum(serialize = "low")]
	Low,
	#[default]
	#[strum(serialize = "medium")]
	Medium,
	#[strum(serialize = "high")]
	High,
	#[strum(serialize = "veryHigh")]
	VeryHigh,
}
impl ToSql<VarChar, diesel::pg::Pg> for SensitiveMediaDetectionSensitivity where String: ToSql<VarChar, diesel::pg::Pg>{
	fn to_sql<'b>(&'b self,out:&mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result{
		<String as ToSql<VarChar, diesel::pg::Pg>>::to_sql(&self.to_string(), &mut out.reborrow())
	}
}
impl<DB: diesel::backend::Backend> FromSql<VarChar, DB> for SensitiveMediaDetectionSensitivity where String: FromSql<VarChar, DB>{
	fn from_sql(bytes: DB::RawValue<'_>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let v=<String as FromSql<VarChar, DB>>::from_sql(bytes)?;
		use std::str::FromStr;
		Self::from_str(&v).or_else(|_|Ok(Self::default()))
	}
}

/*

	@Column('boolean', {
		default: false,
	})
	public enableSensitiveMediaDetectionForVideos: boolean;

	@Column('boolean', {
		default: false,
	})
	public useObjectStorage: boolean;

	@Column('varchar', {
		length: 1024,
		nullable: true,
	})
	public objectStorageBucket: string | null;

	@Column('varchar', {
		length: 1024,
		nullable: true,
	})
	public objectStoragePrefix: string | null;

	@Column('varchar', {
		length: 1024,
		nullable: true,
	})
	public objectStorageBaseUrl: string | null;

	@Column('varchar', {
		length: 1024,
		nullable: true,
	})
	public objectStorageEndpoint: string | null;

	@Column('varchar', {
		length: 1024,
		nullable: true,
	})
	public objectStorageRegion: string | null;

	@Column('varchar', {
		length: 1024,
		nullable: true,
	})
	public objectStorageAccessKey: string | null;

	@Column('varchar', {
		length: 1024,
		nullable: true,
	})
	public objectStorageSecretKey: string | null;

	@Column('integer', {
		nullable: true,
	})
	public objectStoragePort: number | null;

	@Column('boolean', {
		default: true,
	})
	public objectStorageUseSSL: boolean;

	@Column('boolean', {
		default: true,
	})
	public objectStorageUseProxy: boolean;

	@Column('boolean', {
		default: false,
	})
	public objectStorageSetPublicRead: boolean;

	@Column('boolean', {
		default: true,
	})
	public objectStorageS3ForcePathStyle: boolean;

	@Column('boolean', {
		default: false,
	})
	public useObjectStorageRemote: boolean;

	@Column('varchar', {
		length: 1024,
		nullable: true,
	})
	public objectStorageRemoteBucket: string | null;

	@Column('varchar', {
		length: 1024,
		nullable: true,
	})
	public objectStorageRemotePrefix: string | null;

	@Column('varchar', {
		length: 1024,
		nullable: true,
	})
	public objectStorageRemoteBaseUrl: string | null;

	@Column('varchar', {
		length: 1024,
		nullable: true,
	})
	public objectStorageRemoteEndpoint: string | null;

	@Column('varchar', {
		length: 1024,
		nullable: true,
	})
	public objectStorageRemoteRegion: string | null;

	@Column('varchar', {
		length: 1024,
		nullable: true,
	})
	public objectStorageRemoteAccessKey: string | null;

	@Column('varchar', {
		length: 1024,
		nullable: true,
	})
	public objectStorageRemoteSecretKey: string | null;

	@Column('integer', {
		nullable: true,
	})
	public objectStorageRemotePort: number | null;

	@Column('boolean', {
		default: true,
	})
	public objectStorageRemoteUseSSL: boolean;

	@Column('boolean', {
		default: true,
	})
	public objectStorageRemoteUseProxy: boolean;

	@Column('boolean', {
		default: false,
	})
	public objectStorageRemoteSetPublicRead: boolean;

	@Column('boolean', {
		default: true,
	})
	public objectStorageRemoteS3ForcePathStyle: boolean;

	@Column('boolean', {
		default: false,
	})
	public enableIpLogging: boolean;

	@Column('boolean', {
		default: true,
	})
	public enableActiveEmailValidation: boolean;

	@Column('boolean', {
		default: false,
	})
	public enableVerifymailApi: boolean;

	@Column('varchar', {
		length: 1024,
		nullable: true,
	})
	public verifymailAuthKey: string | null;

	@Column('boolean', {
		default: false,
	})
	public enableTruemailApi: boolean;

	@Column('varchar', {
		length: 1024,
		nullable: true,
	})
	public truemailInstance: string | null;

	@Column('varchar', {
		length: 1024,
		nullable: true,
	})
	public truemailAuthKey: string | null;

	@Column('boolean', {
		default: true,
	})
	public enableChartsForRemoteUser: boolean;

	@Column('boolean', {
		default: false,
	})
	public enableServerMachineStats: boolean;

	@Column('boolean', {
		default: true,
	})
	public enableIdenticonGeneration: boolean;

	@Column('jsonb', {
		default: { },
	})
	public policies: Record<string, any>;

	@Column('varchar', {
		length: 280,
		array: true,
		default: '{}',
	})
	public serverRules: string[];

	@Column('varchar', {
		length: 8192,
		default: '{}',
	})
	public manifestJsonOverride: string;

	@Column('varchar', {
		length: 1024,
		array: true,
		default: '{}',
	})
	public bannedEmailDomains: string[];

	@Column('varchar', {
		length: 1024, array: true, default: '{ "admin", "administrator", "root", "system", "maintainer", "host", "mod", "moderator", "owner", "superuser", "staff", "auth", "i", "me", "everyone", "all", "mention", "mentions", "example", "user", "users", "account", "accounts", "official", "help", "helps", "support", "supports", "info", "information", "informations", "announce", "announces", "announcement", "announcements", "notice", "notification", "notifications", "dev", "developer", "developers", "tech", "misskey", "cherrypick" }',
	})
	public preservedUsernames: string[];

	@Column('boolean', {
		default: true,
	})
	public enableFanoutTimeline: boolean;

	@Column('boolean', {
		default: true,
	})
	public enableFanoutTimelineDbFallback: boolean;

	@Column('integer', {
		default: 300,
	})
	public perLocalUserUserTimelineCacheMax: number;

	@Column('integer', {
		default: 100,
	})
	public perRemoteUserUserTimelineCacheMax: number;

	@Column('integer', {
		default: 300,
	})
	public perUserHomeTimelineCacheMax: number;

	@Column('integer', {
		default: 300,
	})
	public perUserListTimelineCacheMax: number;

	@Column('integer', {
		default: 0,
	})
	public notesPerOneAd: number;

	@Column('boolean', {
		default: true,
	})
	public urlPreviewEnabled: boolean;

	@Column('integer', {
		default: 10000,
	})
	public urlPreviewTimeout: number;

	@Column('bigint', {
		default: 1024 * 1024 * 10,
	})
	public urlPreviewMaximumContentLength: number;

	@Column('boolean', {
		default: true,
	})
	public urlPreviewRequireContentLength: boolean;

	@Column('varchar', {
		length: 1024,
		nullable: true,
	})
	public urlPreviewSummaryProxyUrl: string | null;

	@Column('varchar', {
		length: 1024,
		nullable: true,
	})
	public urlPreviewUserAgent: string | null;

	@Column('boolean', {
		default: false,
	})
	public doNotSendNotificationEmailsForAbuseReport: boolean;

	@Column('varchar', {
		length: 1024, nullable: true,
	})
	public emailToReceiveAbuseReport: string | null;

	@Column('boolean', {
		default: false,
	})
	public enableReceivePrerelease: boolean;

	@Column('boolean', {
		default: false,
	})
	public skipVersion: boolean;

	@Column('varchar', {
		length: 32,
		nullable: true,
	})
	public skipCherryPickVersion: string | null;
*/
