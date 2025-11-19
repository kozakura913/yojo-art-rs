use axum::{http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use uuid::uuid;

use crate::{
	Context, ServerError,
	models::{
		self,
		emoji::MiEmoji,
		note::{MiNote, NoteReactionAcceptances, NoteVisibility},
		note_reaction::MiNoteReaction,
		user_profile::MiUserProfile,
	},
	service::{
		activitypub::deliver::DeliverTarget, event::NoteEventType,
		notification::MiReactionNotification, token_service::Token,
	},
};
const FALLBACK: &'static str = "\u{2764}";
const PER_NOTE_REACTION_USER_PAIR_CACHE_MAX: usize = 16;

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
	println!("reaction/create: {:?}", parms);
	let permission = ctx.token_service.get_permission(&parms.i).await;
	let me_id = ServerError::map_opt(
		permission.as_user_id(),
		format!("{}:{}", file!(), line!()),
		"827d7f9b-b64d-4a0a-9769-bd67ae7daff0",
	)?;
	let note = {
		let mut db = ServerError::map_err(
			ctx.raw_db.get_read_only().await,
			"32d57ede-fbd4-44a1-bc4b-01a27f12779c",
		)?;
		let note = models::note::MiNote::load_by_id(&mut db, &parms.note_id);
		ServerError::map_err(note.await, "ba70da04-3fd7-4317-867a-1267af7ba30e")
	}?;
	if note.is_renote() && !note.is_quote() {
		return Err(ServerError::new(
			StatusCode::BAD_REQUEST,
			"You cannot react to Renote.".to_owned(),
			uuid!("f51bff43-eedc-4d0c-b4b8-c269f85caa60"),
		));
	}
	let followings = if note.visibility == NoteVisibility::Followers {
		Some(ServerError::map_err(
			ctx.user_service.followings(&me_id).await,
			"520c0d79-5cae-4fac-8867-7a73616437ff",
		)?)
	} else {
		None
	};
	if !note.is_visible(me_id, followings.as_ref()) {
		return Err(ServerError::new(
			StatusCode::FORBIDDEN,
			"Note not accessible for you.".to_owned(),
			uuid!("664fbe41-9a0e-46eb-8dbc-dd9946ef3a9e"),
		));
	}
	//TODO ユーザーブロックの検証
	let emoji = match note.reaction_acceptance {
		Some(NoteReactionAcceptances::LikeOnly) => Some(FALLBACK), //Likeのみ
		Some(NoteReactionAcceptances::LikeOnlyForRemote) => {
			parms.reaction.as_ref().map(|s| s.as_str()) //リモートユーザーはとりあえず考慮しない
		}
		_ => parms.reaction.as_ref().map(|s| s.as_str()),
	};
	let emoji = emoji.unwrap_or(FALLBACK);
	let mut custom_emoji = None;
	let reaction = if let Some(unicode_emoji) = emojis::get(emoji) {
		//Unicode絵文字
		println!("Unicode絵文字 {:?}", unicode_emoji);
		emoji.to_owned()
	} else {
		//Unicode絵文字ではない
		let emoji = if emoji.starts_with(":") && emoji.ends_with(":") {
			let emoji = MiEmoji::load_local_emoji(
				&mut ServerError::map_err(
					ctx.raw_db.get_read_only().await,
					"b6fd4662-5755-4413-bda4-377ecf0e56a1",
				)?,
				&emoji[1..emoji.len() - 1],
			)
			.await;
			let emoji = ServerError::map_err(emoji, "68118ddb-3615-4ede-aa1f-5d18170970b2")?;
			println!("カスタム絵文字 {:?}", emoji);
			emoji
		} else {
			return Err(ServerError::new(
				StatusCode::BAD_REQUEST,
				"Invalid Emoji".to_owned(),
				uuid!("02b7e5e6-f6b3-4065-ba55-de6669375ac9"),
			));
		};
		custom_emoji = Some(emoji.clone());
		//TODO ロール制限
		if emoji.is_sensitive {
			match note.reaction_acceptance {
				Some(NoteReactionAcceptances::NonSensitiveOnly)
				| Some(NoteReactionAcceptances::NonSensitiveOnlyForLocalLikeOnlyForRemote) => {
					FALLBACK.to_owned()
				}
				_ => format!(":{}:", emoji.name),
			}
		} else {
			format!(":{}:", emoji.name)
		}
	};
	let reaction = MiNoteReaction {
		id: ctx.id_service.gen_id(None),
		note_id: note.id.clone(),
		user_id: me_id.clone(),
		reaction,
	};
	let old_reaction = MiNoteReaction::load_by_user_note(
		&mut ServerError::map_err(
			ctx.raw_db.get_read_only().await,
			"f7138569-ae10-4158-8d88-ca0d66e18ea9",
		)?,
		&reaction.user_id,
		&reaction.note_id,
	)
	.await;
	let old_reaction = ServerError::map_err(old_reaction, "56c97545-11ca-492e-b00f-6ad67b92f1a8")?;
	if let Some(old_reaction) = old_reaction.get(0) {
		if old_reaction.reaction == reaction.reaction {
			return Err(ServerError::new(
				StatusCode::BAD_REQUEST,
				"alreadyReacted".to_owned(),
				uuid!("8b14c9f9-2cb0-40a6-b043-c40bfdd90ed3"),
			));
		}
	}
	{
		let db_start = chrono::Utc::now();
		//DBに投入
		let mut con = ServerError::map_err(
			ctx.raw_db.get_writeable().await,
			"f058be45-86b3-4228-b701-74cd5b92e1e7",
		)?;
		use diesel_async::AsyncConnection;
		let res = con
			.transaction(|con| {
				Box::pin(async {
					println!(
						"トランザクション開始{}ms",
						(chrono::Utc::now() - db_start).num_milliseconds()
					);
					let old_note = MiNote::load_by_id(con, &reaction.note_id).await?;
					println!(
						"旧ノート取得{}ms",
						(chrono::Utc::now() - db_start).num_milliseconds()
					);
					let mut update_reactions = old_note.reactions;
					let mut react_count = update_reactions
						.0
						.get(&reaction.reaction)
						.copied()
						.unwrap_or(0);
					react_count += 1;
					update_reactions
						.0
						.insert(reaction.reaction.to_owned(), react_count);
					let mut reaction_cache = old_note.reaction_and_user_pair_cache;
					if reaction_cache.len() < PER_NOTE_REACTION_USER_PAIR_CACHE_MAX {
						reaction_cache.push(format!("{}/{}", reaction.user_id, &reaction.reaction));
					}
					println!(
						"差分生成{}ms",
						(chrono::Utc::now() - db_start).num_milliseconds()
					);
					{
						use crate::models::note::note::dsl::note;
						use crate::models::note::note::dsl::*;
						use diesel::{ExpressionMethods, QueryDsl};
						use diesel_async::RunQueryDsl;
						diesel::update(note.filter(id.eq(&reaction.note_id)))
							.set((
								reactions.eq(update_reactions),
								reactionAndUserPairCache.eq(reaction_cache),
							))
							.execute(con)
							.await?;
					}
					println!(
						"ノート投入{}ms",
						(chrono::Utc::now() - db_start).num_milliseconds()
					);
					reaction.insert_into(con).await?;
					println!(
						"リアクション投入{}ms",
						(chrono::Utc::now() - db_start).num_milliseconds()
					);
					diesel::result::QueryResult::Ok(())
				})
			})
			.await;
		ServerError::map_err(res, "f8dadde8-10fc-4774-bae6-9b5cbaea7725")?;
		println!(
			"DB投入{}ms",
			(chrono::Utc::now() - db_start).num_milliseconds()
		);
	}
	//TODO OpenSearchにindexする
	//TODO チャート更新
	#[derive(Debug, Deserialize, Serialize)]
	struct EmojiUrl {
		name: String,
		url: String,
	}
	#[derive(Debug, Deserialize, Serialize)]
	struct EventBody {
		reaction: String,
		#[serde(skip_serializing_if = "Option::is_none")]
		emoji: Option<EmojiUrl>,
		#[serde(rename = "userId")]
		user_id: String,
	}
	let normalize_reaction = ctx
		.emoji_service
		.normalize_reaction(reaction.reaction.to_owned());
	let event_body = EventBody {
		reaction: normalize_reaction.clone(),
		emoji: custom_emoji.as_ref().map(|e| EmojiUrl {
			name: normalize_reaction,
			url: e.public_url.clone(),
		}),
		user_id: me_id.clone(),
	};
	let event_body = ServerError::map_err(
		serde_json::to_value(event_body),
		"50637b33-ca4b-4378-9f63-6a42d69c7ed6",
	)?;
	ServerError::map_err(
		ctx.event_service
			.publish_note_stream(
				&reaction.note_id,
				Some(NoteEventType::NoteUpdated),
				event_body,
			)
			.await,
		"e2710f7b-7916-4013-8c06-40d532b6e9a7",
	)?;
	//TODO 通知の作成
	ctx.notification_service
		.create_notification(
			Some(me_id.clone()),
			&note.user_id,
			MiReactionNotification::new(reaction.note_id.clone(), reaction.reaction.clone()),
		)
		.await;

	// 2秒経っても(今回作成した)通知が既読にならなかったら「未読の通知がありますよ」イベントを発行する
	tokio::runtime::Handle::current().spawn(async {
		tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
		//TODO イベント発行
	});
	let content = ctx
		.ap_render_service
		.render_like(&note, reaction.clone(), custom_emoji.take());
	let content = ctx.ap_render_service.add_context(content)?;
	//メンション、宛先ユーザーにも配送した方が良いかも
	if note.user_host.is_none() {
		//ノート作者がローカルユーザーならフォロワーのみ
		let target = [DeliverTarget::Follower];
		ServerError::map_err(
			ctx.deliver_service
				.post(target.into_iter(), reaction.user_id, content)
				.await,
			"f80d98c8-d587-40be-bd92-f40165845a3c",
		)?;
	} else {
		//ノート作者にも配送
		let target = [DeliverTarget::Direct(note.user_id), DeliverTarget::Follower];
		ServerError::map_err(
			ctx.deliver_service
				.post(target.into_iter(), reaction.user_id, content)
				.await,
			"ef29621e-f777-4b6f-9b9b-9695be359d04",
		)?;
	}
	Ok(StatusCode::NO_CONTENT.into_response())
}
