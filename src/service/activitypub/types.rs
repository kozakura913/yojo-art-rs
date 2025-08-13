use serde::{Deserialize, Serialize};

use crate::models::emoji::EmojiCopyPermissions;

#[derive(Debug, Deserialize, Serialize)]
pub struct FreeText {
	#[serde(rename = "@id")]
	pub id: &'static str,
	#[serde(rename = "@type")]
	pub ap_type: &'static str,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ApSearchableBy {
	#[serde(rename = "@id")]
	pub id: &'static str,
	#[serde(rename = "@type")]
	pub ap_type: &'static str,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ApImage {
	#[serde(rename = "type")]
	pub ap_type: &'static str,
	#[serde(rename = "mediaType")]
	pub media_type: String,
	pub url: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ApMisskeyLicense {
	#[serde(rename = "freeText")]
	pub free_text: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ApEmojiAuthor {
	pub author: String,
	pub creator: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ApEmoji {
	pub id: String,
	#[serde(rename = "type")]
	pub ap_type: &'static str,
	pub name: String,
	pub updated: String,
	pub icon: ApImage,
	pub _misskey_license: ApMisskeyLicense,
	pub keywords: String,
	#[serde(rename = "isSensitive")]
	pub is_sensitive: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "copyPermission")]
	pub copy_permission: Option<EmojiCopyPermissions>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub category: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub license: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "usageInfo")]
	pub usage_info: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "isBasedOn")]
	pub is_based_on: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub author: Option<ApEmojiAuthor>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApLike {
	#[serde(rename = "type")]
	pub ap_type: &'static str,
	pub id: String,
	pub actor: String,
	pub object: String,
	pub content: String,
	pub _misskey_reaction: String,
	pub tag: Vec<ApEmoji>,
}
