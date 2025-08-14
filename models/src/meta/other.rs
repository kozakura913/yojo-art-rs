use diesel::{
	Selectable,
	deserialize::{FromSql, FromSqlRow},
	expression::AsExpression,
	serialize::ToSql,
	sql_types::VarChar,
};
diesel::table! {
	meta (id) {
		id -> VarChar,
		cacheRemoteFiles -> Bool,
		cacheRemoteSensitiveFiles -> Bool,
		proxyAccountId -> Nullable<VarChar>,
		sensitiveMediaDetection -> VarChar,
		sensitiveMediaDetectionSensitivity -> VarChar,
		enableSensitiveMediaDetectionForVideos -> Bool,
		enableIpLogging -> Bool,
		policies -> Jsonb,
		setSensitiveFlagAutomatically -> Bool,
		enableChartsForFederatedInstances -> Bool,
		enableFanoutTimeline -> Bool,
		enableFanoutTimelineDbFallback -> Bool,
	}
}

#[derive(
	PartialEq,
	Eq,
	Debug,
	Clone,
	diesel::Insertable,
	diesel::Queryable,
	Selectable,
	diesel::QueryableByName,
)]
#[diesel(table_name = meta)]
pub struct MiMetaOther {
	pub id: String,
	#[diesel(column_name = "cacheRemoteFiles")]
	pub cache_remote_files: bool,
	#[diesel(column_name = "cacheRemoteSensitiveFiles")]
	pub cache_remote_sensitive_files: bool,
	#[diesel(column_name = "proxyAccountId")]
	pub proxy_account_id: Option<String>,
	#[diesel(column_name = "sensitiveMediaDetection")]
	pub sensitive_media_detection: SensitiveMediaDetection,
	#[diesel(column_name = "sensitiveMediaDetectionSensitivity")]
	pub sensitive_media_detection_sensitivity: SensitiveMediaDetectionSensitivity,
	#[diesel(column_name = "enableSensitiveMediaDetectionForVideos")]
	pub enable_sensitive_media_detection_for_videos: bool,
	#[diesel(column_name = "enableIpLogging")]
	pub enable_ip_logging: bool,
	pub policies: serde_json::Value,
	#[diesel(column_name = "setSensitiveFlagAutomatically")]
	pub set_sensitive_flag_automatically: bool,
	#[diesel(column_name = "enableChartsForFederatedInstances")]
	pub enable_charts_for_federated_instances: bool,
	#[diesel(column_name = "enableFanoutTimeline")]
	pub enable_fanout_timeline: bool,
	#[diesel(column_name = "enableFanoutTimelineDbFallback")]
	pub enable_fanout_timeline_db_fallback: bool,
}

#[derive(
	PartialEq,
	Eq,
	Copy,
	Clone,
	strum_macros::EnumString,
	strum_macros::Display,
	Default,
	Debug,
	FromSqlRow,
	AsExpression,
)]
#[diesel(sql_type = VarChar)]
pub enum SensitiveMediaDetection {
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
impl ToSql<VarChar, diesel::pg::Pg> for SensitiveMediaDetection
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
impl<DB: diesel::backend::Backend> FromSql<VarChar, DB> for SensitiveMediaDetection
where
	String: FromSql<VarChar, DB>,
{
	fn from_sql(bytes: DB::RawValue<'_>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let v = <String as FromSql<VarChar, DB>>::from_sql(bytes)?;
		use std::str::FromStr;
		Self::from_str(&v).or_else(|_| Ok(Self::default()))
	}
}
#[derive(
	PartialEq,
	Eq,
	Copy,
	Clone,
	strum_macros::EnumString,
	strum_macros::Display,
	Default,
	Debug,
	FromSqlRow,
	AsExpression,
)]
#[diesel(sql_type = VarChar)]
pub enum SensitiveMediaDetectionSensitivity {
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
impl ToSql<VarChar, diesel::pg::Pg> for SensitiveMediaDetectionSensitivity
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
impl<DB: diesel::backend::Backend> FromSql<VarChar, DB> for SensitiveMediaDetectionSensitivity
where
	String: FromSql<VarChar, DB>,
{
	fn from_sql(bytes: DB::RawValue<'_>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let v = <String as FromSql<VarChar, DB>>::from_sql(bytes)?;
		use std::str::FromStr;
		Self::from_str(&v).or_else(|_| Ok(Self::default()))
	}
}
