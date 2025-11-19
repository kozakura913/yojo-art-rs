use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{
	MisskeyConfig, ServerError,
	models::{emoji::MiEmoji, note::MiNote, note_reaction::MiNoteReaction},
	service::activitypub::types::{
		ApEmoji, ApEmojiAuthor, ApImage, ApLike, ApMisskeyLicense, ApSearchableBy, FreeText,
	},
};

#[derive(Debug, Deserialize, Serialize)]
pub struct ApContext {
	#[serde(rename = "Key")]
	key: &'static str,
	#[serde(rename = "manuallyApprovesFollowers")]
	manually_approves_followers: &'static str,
	sensitive: &'static str,
	#[serde(rename = "Hashtag")]
	hashtag: &'static str,
	#[serde(rename = "quoteUrl")]
	quote_url: &'static str,
	toot: &'static str,
	#[serde(rename = "Emoji")]
	emoji: &'static str,
	featured: &'static str,
	discoverable: &'static str,
	indexable: &'static str,
	fedibird: &'static str,
	#[serde(rename = "searchableBy")]
	searchable_by: ApSearchableBy,
	schema: &'static str,
	#[serde(rename = "PropertyValue")]
	property_value: &'static str,
	value: &'static str,
	misskey: &'static str,
	#[serde(rename = "_misskey_content")]
	misskey_content: &'static str,
	#[serde(rename = "_misskey_quote")]
	misskey_quote: &'static str,
	#[serde(rename = "_misskey_reaction")]
	misskey_reaction: &'static str,
	#[serde(rename = "_misskey_votes")]
	misskey_votes: &'static str,
	#[serde(rename = "_misskey_summary")]
	misskey_summary: &'static str,
	#[serde(rename = "_misskey_followedMessage")]
	misskey_followed_message: &'static str,
	#[serde(rename = "_misskey_requireSigninToViewContents")]
	misskey_require_signin_to_view_contents: &'static str,
	#[serde(rename = "_misskey_makeNotesFollowersOnlyBefore")]
	misskey_make_notes_followers_only_before: &'static str,
	#[serde(rename = "_misskey_makeNotesHiddenBefore")]
	misskey_make_notes_hidden_before: &'static str,
	#[serde(rename = "_misskey_license")]
	misskey_license: &'static str,
	#[serde(rename = "freeText")]
	free_text: FreeText,
	#[serde(rename = "_misskey_talk")]
	misskey_talk: &'static str,
	#[serde(rename = "isCat")]
	is_cat: &'static str,
	yojoart: &'static str,
	banner: &'static str,
	#[serde(rename = "Game")]
	game: &'static str,
	#[serde(rename = "_yojoart_clips")]
	yojoart_clips: &'static str,
	vcard: &'static str,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ApBody {
	id: String,
	#[serde(rename = "@context")]
	context: serde_json::Value,
}
#[derive(Clone)]
pub struct ApRenderService {
	misskey_config: Arc<MisskeyConfig>,
}
impl ApRenderService {
	pub fn new(misskey_config: Arc<MisskeyConfig>) -> Self {
		Self { misskey_config }
	}
	pub fn render_emoji(&self, emoji: MiEmoji) -> ApEmoji {
		ApEmoji {
			id: format!("{}/emojis/{}", self.misskey_config.url, emoji.name),
			ap_type: "Emoji",
			name: emoji.name,
			updated: emoji
				.updated_at
				.map(|t| t.and_utc())
				.unwrap_or_else(|| chrono::Utc::now())
				.to_rfc3339(),
			icon: ApImage {
				ap_type: "Image",
				media_type: emoji.image_type.unwrap_or_else(|| "image/png".into()),
				url: emoji.public_url,
			},
			_misskey_license: ApMisskeyLicense {
				free_text: emoji.license.clone(),
			},
			keywords: emoji.aliases.iter().fold(String::new(), |a, b| a + b),
			is_sensitive: emoji.is_sensitive,
			copy_permission: emoji.copy_permission,
			category: emoji.category,
			license: emoji.license,
			usage_info: emoji.usage_info,
			description: emoji.description,
			is_based_on: emoji.is_based_on,
			author: emoji.author.map(|author| ApEmojiAuthor {
				creator: author.clone(),
				author,
			}),
		}
	}
	pub fn render_like(
		&self,
		note: &MiNote,
		reaction: MiNoteReaction,
		emoji: Option<MiEmoji>,
	) -> ApLike {
		assert_eq!(note.id, reaction.note_id);
		let mut tag = vec![];
		if let Some(custom_emoji) = emoji {
			tag.push(self.render_emoji(custom_emoji));
		}
		ApLike {
			ap_type: "Like",
			id: format!("{}likes/{}", self.misskey_config.url, reaction.id),
			actor: format!("{}users/{}", self.misskey_config.url, reaction.user_id),
			object: note.uri.clone().unwrap_or_else(|| {
				format!("{}notes/{}", self.misskey_config.url, reaction.note_id)
			}),
			content: reaction.reaction.to_owned(),
			_misskey_reaction: reaction.reaction.to_owned(),
			tag,
		}
	}
	pub fn add_context(
		&self,
		json: impl Serialize,
	) -> Result<serde_json::Value, crate::ServerError> {
		let mut value = ServerError::map_err(
			serde_json::to_value(json),
			"720f2ced-39a1-4a09-81a0-1be02e6552bb",
		)?;
		let value_object = ServerError::map_opt(
			value.as_object_mut(),
			"ap",
			"eb086837-257c-4335-8764-6ffca78d18ea",
		)?;
		let mut context = vec![];
		context.push(serde_json::Value::String(
			"https://www.w3.org/ns/activitystreams".into(),
		));
		context.push(serde_json::Value::String(
			"https://w3id.org/security/v1".into(),
		));
		context.push(
			serde_json::to_value(ApContext {
				key: "sec:Key",
				manually_approves_followers: "as:manuallyApprovesFollowers",
				sensitive: "as:sensitive",
				hashtag: "as:Hashtag",
				quote_url: "as:quoteUrl",
				toot: "http://joinmastodon.org/ns#",
				emoji: "toot:Emoji",
				featured: "toot:featured",
				discoverable: "toot:discoverable",
				indexable: "toot:indexable",
				fedibird: "http://fedibird.com/ns#",
				searchable_by: ApSearchableBy {
					id: "fedibird:searchableBy",
					ap_type: "@id",
				},
				schema: "http://schema.org#",
				property_value: "schema:PropertyValue",
				value: "schema:value",
				misskey: "https://misskey-hub.net/ns#",
				misskey_content: "misskey:_misskey_content",
				misskey_quote: "misskey:_misskey_quote",
				misskey_reaction: "misskey:_misskey_reaction",
				misskey_votes: "misskey:_misskey_votes",
				misskey_summary: "misskey:_misskey_summary",
				misskey_followed_message: "misskey:_misskey_followedMessage",
				misskey_require_signin_to_view_contents: "misskey:_misskey_requireSigninToViewContents",
				misskey_make_notes_followers_only_before: "misskey:_misskey_makeNotesFollowersOnlyBefore",
				misskey_make_notes_hidden_before: "misskey:_misskey_makeNotesHiddenBefore",
				misskey_license: "misskey:_misskey_license",
				free_text: FreeText {
					id: "misskey:freeText",
					ap_type: "schema:text",
				},
				misskey_talk: "misskey:_misskey_talk",
				is_cat: "misskey:isCat",
				yojoart: "https://yojoart.kzkr.xyz/ns#",
				banner: "yojoart:banner",
				game: "yojoart:Game",
				yojoart_clips: "yojoart:_yojoart_clips",
				vcard: "http://www.w3.org/2006/vcard/ns#",
			})
			.unwrap(),
		);
		let context = ApBody {
			id: format!("{}/{}", self.misskey_config.url, uuid::Uuid::new_v4()),
			context: serde_json::Value::Array(context),
		};
		let mut ap_context = ServerError::map_err(
			serde_json::to_value(context),
			"2b44c26d-01f4-4045-88d0-61ffd379a44a",
		)?;
		value_object.append(ServerError::map_opt(
			ap_context.as_object_mut(),
			"ap",
			"41f95272-631d-4893-a889-d1dea62f2cc5",
		)?);
		Ok(value)
	}
}
