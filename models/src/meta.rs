pub mod branding;
pub mod moderation;
pub mod other;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct MiMeta {
	pub other: other::MiMetaOther,
	pub branding: branding::MiMetaBranding,
	pub moderation: moderation::MiMetaModeration,
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
