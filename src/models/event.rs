use chrono::NaiveDateTime;
use diesel::{
	ExpressionMethods, FromSqlRow, QueryDsl, Selectable, SelectableHelper,
	deserialize::FromSql,
	expression::AsExpression,
	serialize::{IsNull, ToSql},
	sql_types::{Jsonb, Nullable, VarChar},
};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use yojo_art_utils::{PgJson, PgString};

use super::note::NoteVisibility;

diesel::table! {
	#[sql_name = "event"]
	event (noteId) {
		noteId -> VarChar,
		start -> Timestamp,
		end -> Nullable<Timestamp>,
		title -> VarChar,
		metadata -> Jsonb,
		noteVisibility -> crate::models::note::NoteVisibilityType,
		userId -> VarChar,
		userHost -> Nullable<VarChar>,
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
#[diesel(table_name = event)]
pub struct MiEvent {
	#[diesel(column_name = "noteId")]
	pub note_id: String,
	pub start: NaiveDateTime,
	pub end: Option<NaiveDateTime>,
	pub title: String,
	pub metadata: EventMetadata,
	#[diesel(column_name = "noteVisibility")]
	pub note_visibility: NoteVisibility,
	#[diesel(column_name = "userId")]
	pub user_id: String,
	#[diesel(column_name = "userHost")]
	pub user_host: Option<String>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize, FromSqlRow, AsExpression, PgJson)]
#[diesel(sql_type = Jsonb)]
pub struct EventMetadata {
	#[serde(rename = "@type")]
	ap_type: String, //Event
	#[serde(skip_serializing_if = "Option::is_none")]
	name: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	url: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	description: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	audience: Option<EventAudience>,
	#[serde(rename = "doorTime")]
	door_time: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "startDate")]
	start_date: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "endDate")]
	end_date: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "eventStatus")]
	event_status: Option<EventStatus>,
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "inLanguage")]
	in_language: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "isAccessibleForFree")]
	is_accessible_for_free: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	keywords: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	location: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	offers: Option<EventOffer>,
	#[serde(skip_serializing_if = "Option::is_none")]
	organizer: Option<EventOrganizer>,
	#[serde(skip_serializing_if = "Option::is_none")]
	performer: Option<EventPerformer>,
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "typicalAgeRange")]
	typical_age_range: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	identifier: Option<String>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct EventPerformer {
	name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "sameAs")]
	same_as: Option<String>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct EventOrganizer {
	name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "sameAs")]
	same_as: Option<String>,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct EventOffer {
	#[serde(rename = "@type")]
	ap_type: String, //Offer
	#[serde(skip_serializing_if = "Option::is_none")]
	price: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "priceCurrency")]
	price_currency: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "availabilityStarts")]
	availability_starts: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "availabilityEnds")]
	availability_ends: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	url: Option<String>,
}
#[derive(PartialEq, Eq, Copy, Clone, EnumString, Display, Debug, Serialize, Deserialize)]
pub enum EventStatus {
	#[strum(serialize = "https://schema.org/EventCancelled")]
	Cancelled,
	#[strum(serialize = "https://schema.org/EventMovedOnline")]
	MovedOnline,
	#[strum(serialize = "https://schema.org/EventPostponed")]
	Postponed,
	#[strum(serialize = "https://schema.org/EventRescheduled")]
	Rescheduled,
	#[strum(serialize = "https://schema.org/EventScheduled")]
	Scheduled,
}
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct EventAudience {
	#[serde(rename = "@type")]
	ap_type: String, //Audience
	name: String,
}
