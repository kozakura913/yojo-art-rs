use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize,Serialize)]
pub struct FreeText{
	#[serde(rename = "@id")]
	id:&'static str,
	#[serde(rename = "@type")]
	ap_type: &'static str,
}
#[derive(Debug, Deserialize,Serialize)]
pub struct ApSearchableBy{
	#[serde(rename = "@id")]
	id:&'static str,
	#[serde(rename = "@type")]
	ap_type: &'static str,
}
#[derive(Debug, Deserialize,Serialize)]
pub struct ApContext{
	#[serde(rename = "Key")]
	key:&'static str,
	#[serde(rename = "manuallyApprovesFollowers")]
	manually_approves_followers:&'static str,
	sensitive:&'static str,
	#[serde(rename = "Hashtag")]
	hashtag:&'static str,
	#[serde(rename = "quoteUrl")]
	quote_url:&'static str,
	toot:&'static str,
	#[serde(rename = "Emoji")]
	emoji:&'static str,
	featured:&'static str,
	discoverable:&'static str,
	indexable:&'static str,
	fedibird:&'static str,
	#[serde(rename = "searchableBy")]
	searchable_by:ApSearchableBy,
	schema:&'static str,
	#[serde(rename = "PropertyValue")]
	property_value:&'static str,
	value:&'static str,
	misskey:&'static str,
	#[serde(rename = "_misskey_content")]
	misskey_content:&'static str,
	#[serde(rename = "_misskey_quote")]
	misskey_quote:&'static str,
	#[serde(rename = "_misskey_reaction")]
	misskey_reaction:&'static str,
	#[serde(rename = "_misskey_votes")]
	misskey_votes:&'static str,
	#[serde(rename = "_misskey_summary")]
	misskey_summary:&'static str,
	#[serde(rename = "_misskey_followedMessage")]
	misskey_followed_message:&'static str,
	#[serde(rename = "_misskey_requireSigninToViewContents")]
	misskey_require_signin_to_view_contents:&'static str,
	#[serde(rename = "_misskey_makeNotesFollowersOnlyBefore")]
	misskey_make_notes_followers_only_before:&'static str,
	#[serde(rename = "_misskey_makeNotesHiddenBefore")]
	misskey_make_notes_hidden_before:&'static str,
	#[serde(rename = "_misskey_license")]
	misskey_license:&'static str,
	#[serde(rename = "freeText")]
	free_text:FreeText,
	#[serde(rename = "_misskey_talk")]
	misskey_talk:&'static str,
	#[serde(rename = "isCat")]
	is_cat:&'static str,
	yojoart:&'static str,
	banner:&'static str,
	#[serde(rename = "Game")]
	game:&'static str,
	#[serde(rename = "_yojoart_clips")]
	yojoart_clips:&'static str,
	vcard:&'static str,
}
#[derive(Debug, Deserialize,Serialize)]
pub struct ApBody{
	id:String,
	#[serde(rename = "@context")]
	context:serde_json::Value,
}
pub fn add_context(json:impl Serialize,ctx:&crate::Context)->Result<serde_json::Value,crate::ServerError>{
	let mut value=serde_json::to_value(json)?;
	let value_object=value.as_object_mut().ok_or("eb086837-257c-4335-8764-6ffca78d18ea")?;
	let mut context=vec![];
	context.push(serde_json::Value::String("https://www.w3.org/ns/activitystreams".into()));
	context.push(serde_json::Value::String("https://w3id.org/security/v1".into()));
	context.push(serde_json::to_value(ApContext{
		key:"sec:Key",
		manually_approves_followers:"as:manuallyApprovesFollowers",
		sensitive: "as:sensitive",
		hashtag: "as:Hashtag",
		quote_url:"as:quoteUrl",
		toot:"http://joinmastodon.org/ns#",
		emoji:"toot:Emoji",
		featured:"toot:featured",
		discoverable:"toot:discoverable",
		indexable:"toot:indexable",
		fedibird:"http://fedibird.com/ns#",
		searchable_by:ApSearchableBy{
			id:"fedibird:searchableBy",
			ap_type:"@id",
		},
		schema:"http://schema.org#",
		property_value:"schema:PropertyValue",
		value:"schema:value",
		misskey:"https://misskey-hub.net/ns#",
		misskey_content:"misskey:_misskey_content",
		misskey_quote:"misskey:_misskey_quote",
		misskey_reaction: "misskey:_misskey_reaction",
		misskey_votes:"misskey:_misskey_votes",
		misskey_summary:"misskey:_misskey_summary",
		misskey_followed_message:"misskey:_misskey_followedMessage",
		misskey_require_signin_to_view_contents:"misskey:_misskey_requireSigninToViewContents",
		misskey_make_notes_followers_only_before:"misskey:_misskey_makeNotesFollowersOnlyBefore",
		misskey_make_notes_hidden_before:"misskey:_misskey_makeNotesHiddenBefore",
		misskey_license:"misskey:_misskey_license",
		free_text:FreeText{
			id:  "misskey:freeText",
			ap_type: "schema:text",
		},
		misskey_talk:"misskey:_misskey_talk",
		is_cat:"misskey:isCat",
		yojoart:"https://yojoart.kzkr.xyz/ns#",
		banner:"yojoart:banner",
		game:"yojoart:Game",
		yojoart_clips:"yojoart:_yojoart_clips",
		vcard:"http://www.w3.org/2006/vcard/ns#",
	})?);
	let context=ApBody{
		id:format!("{}/{}",ctx.misskey_config.url,uuid::Uuid::new_v4()),
		context:serde_json::Value::Array(context),
	};
	let mut ap_context=serde_json::to_value(context)?;
	value_object.append(ap_context.as_object_mut().ok_or("41f95272-631d-4893-a889-d1dea62f2cc5")?);
	Ok(value)
}
