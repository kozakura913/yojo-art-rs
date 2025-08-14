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
		disableRegistration -> Bool,
		hiddenTags -> Array<VarChar>,
		blockedHosts -> Array<VarChar>,
		sensitiveWords -> Array<VarChar>,
		prohibitedWords -> Array<VarChar>,
		silencedHosts -> Array<VarChar>,
		mediaSilencedHosts -> Array<VarChar>,
		emailRequiredForSignup -> Bool,
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
pub struct MiMetaModeration {
	pub id: String,
	#[diesel(column_name = "disableRegistration")]
	pub disable_registration: bool,
	#[diesel(column_name = "hiddenTags")]
	pub hidden_tags: Vec<String>,
	#[diesel(column_name = "blockedHosts")]
	pub blocked_hosts: Vec<String>,
	#[diesel(column_name = "sensitiveWords")]
	pub sensitive_words: Vec<String>,
	#[diesel(column_name = "prohibitedWords")]
	pub prohibited_words: Vec<String>,
	#[diesel(column_name = "silencedHosts")]
	pub silenced_wosts: Vec<String>,
	#[diesel(column_name = "mediaSilencedHosts")]
	pub media_silenced_hosts: Vec<String>,
	#[diesel(column_name = "emailRequiredForSignup")]
	pub email_required_for_signup: bool,
}
