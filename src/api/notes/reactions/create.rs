use std::{collections::HashMap, sync::Arc};

use axum::{http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{
	models::{self, emoji::MiEmoji, note::{MiNote, NoteReactionAcceptances, NoteVisibility}, note_reaction::MiNoteReaction}, service::{
		event::NoteEventType, timeline::{TLOptions, TimelineHints}, token_service::Token
	}, Context, ServerError
};
const FALLBACK:&'static str = "\u{2764}";
const PER_NOTE_REACTION_USER_PAIR_CACHE_MAX:usize=16;

#[derive(Debug, Deserialize)]
pub struct RequestParams {
	i: Token, //トークン必須
	#[serde(rename = "noteId")]
	note_id: String,
	reaction: Option<String>,
}
pub async fn post(
	axum::extract::State(ctx): axum::extract::State<std::sync::Arc<Context>>,
	axum::extract::Json(parms): axum::extract::Json<RequestParams>,
) -> Result<axum::response::Response, ServerError> {
	println!("reaction/create: {:?}",parms);
	let permission=ctx.token_service.get_permission(&parms.i).await;
	let me=permission.as_user_id().ok_or(format!("{}:{}",file!(),line!()))?;
	let note=models::note::MiNote::load_by_id(&mut ctx.raw_db.get_read_only().await?,&parms.note_id).await?;
	if note.is_renote()&&!note.is_quote(){
		return Err(ServerError::new(StatusCode::BAD_REQUEST,"You cannot react to Renote.".to_owned()));
	}
	let followings=if note.visibility==NoteVisibility::Followers{
		Some(ctx.user_service.followings(&me).await?)
	}else{
		None
	};
	if !note.is_visible(me,followings.as_ref()){
		return Err(ServerError::new(StatusCode::FORBIDDEN,"Note not accessible for you.".to_owned()));
	}
	//TODO ユーザーブロックの検証
	let emoji=match note.reaction_acceptance{
		Some(NoteReactionAcceptances::LikeOnly) => Some(FALLBACK),//Likeのみ
		Some(NoteReactionAcceptances::LikeOnlyForRemote) => {
			parms.reaction.as_ref().map(|s|s.as_str()) //リモートユーザーはとりあえず考慮しない
		},
		_ => parms.reaction.as_ref().map(|s|s.as_str()),
	};
	let emoji=emoji.unwrap_or(FALLBACK);
	let mut custom_emoji=None;
	let reaction=if let Some(unicode_emoji)=emojis::get(emoji){
		//Unicode絵文字
		println!("Unicode絵文字 {:?}",unicode_emoji);
		emoji.to_owned()
	}else{
		//Unicode絵文字ではない
		let emoji=if emoji.starts_with(":")&&emoji.ends_with(":"){
			let emoji=MiEmoji::load_local_emoji(&mut ctx.raw_db.get_read_only().await?,&emoji[1..emoji.len() - 1]).await?;
			println!("カスタム絵文字 {:?}",emoji);
			emoji
		}else{
			return Err(ServerError::new(StatusCode::BAD_REQUEST,"Invalid Emoji".to_owned()));
		};
		custom_emoji=Some(emoji.clone());
		//TODO ロール制限
		if emoji.is_sensitive{
			match note.reaction_acceptance{
				Some(NoteReactionAcceptances::NonSensitiveOnly)|Some(NoteReactionAcceptances::NonSensitiveOnlyForLocalLikeOnlyForRemote) => {
					FALLBACK.to_owned()
				},
				_ => format!(":{}:",emoji.name),
			}
		}else{
			format!(":{}:",emoji.name)
		}
	};
	let reaction=MiNoteReaction{
		id:ctx.id_service.gen_id(None),
		note_id: note.id,
		user_id:me.clone(),
		reaction,
	};
	{
		let mut con=ctx.raw_db.get_writeable().await?;
		use diesel_async::AsyncConnection;
		con.transaction(|con| Box::pin(async{
			use diesel::{ExpressionMethods,QueryDsl,SelectableHelper};
			use diesel_async::RunQueryDsl;
			{
				use crate::models::note::note::dsl::note;
				use crate::models::note::note::dsl::*;
				let old_note:MiNote=note.filter(id.eq(&reaction.note_id))
					.select(MiNote::as_select())
					.first(con)
					.await?;
				let mut update_reactions=old_note.reactions;
				println!("old update_reactions {:?}",update_reactions);
				let mut react_count=update_reactions.0.get(&reaction.reaction).copied().unwrap_or(0);
				react_count+=1;
				update_reactions.0.insert(reaction.reaction.to_owned(), react_count);
				let mut reaction_cache=old_note.reaction_and_user_pair_cache;
				println!("old reaction_cache {:?}",reaction_cache);
				if reaction_cache.len()<PER_NOTE_REACTION_USER_PAIR_CACHE_MAX{
					reaction_cache.push(format!("{}/{}",reaction.user_id,&reaction.reaction));
				}
				diesel::update(note.filter(id.eq(&reaction.note_id)))
					.set((reactions.eq(update_reactions),reactionAndUserPairCache.eq(reaction_cache)))
					.execute(con).await?;
			}
			let new_reaction=&reaction;
			{
				use crate::models::note_reaction::note_reaction::dsl::note_reaction;
				diesel::insert_into(note_reaction).values(new_reaction)
					.execute(con).await?;
			}
			diesel::result::QueryResult::Ok(())
		})).await?;
	}
	//TODO DBに投入
	//TODO OpenSearchにindexする
	//TODO ノート側のリアクションキャッシュとリアクション数のカウントを進める
	//TODO チャート更新
	#[derive(Debug, Deserialize,Serialize)]
	struct EmojiUrl{
		name:String,
		url:String,
	}
	#[derive(Debug, Deserialize,Serialize)]
	struct EventBody{
		reaction:String,
		#[serde(skip_serializing_if = "Option::is_none")]
		emoji:Option<EmojiUrl>,
		#[serde(rename = "userId")]
		user_id:String,
	}
	let normalize_reaction=ctx.emoji_service.normalize_reaction(reaction.reaction.to_owned());
	let event_body=EventBody{
		reaction:normalize_reaction.clone(),
		emoji:custom_emoji.map(|e|EmojiUrl { name: normalize_reaction, url: e.public_url }),
		user_id:me.clone(),
	};
	let event_body=serde_json::to_value(event_body)?;
	ctx.event_service.publish_note_stream(&reaction.note_id,Some(NoteEventType::NoteUpdated), event_body).await?;
	//TODO 通知の作成
	//TODO 配送
	Ok(StatusCode::NO_CONTENT.into_response())
}
