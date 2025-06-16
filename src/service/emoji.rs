use std::{borrow::Cow, collections::HashMap, sync::Arc, time::Duration};

use memory_cache::MemoryCache;
use tokio::sync::{Mutex, RwLock};

use crate::{DBConnection, DataBase, models::emoji::MiEmoji};
#[derive(Clone, Debug)]
pub struct ParsedEmoji {
	name: Option<String>,
	host: Option<String>,
}
impl TryInto<String> for &ParsedEmoji {
	type Error = ();

	fn try_into(self) -> Result<String, Self::Error> {
		match (self.name.as_ref(), self.host.as_ref()) {
			(None, _) => Err(()),
			(Some(name), None) => Ok(format!("{}@.", name)),
			(Some(name), Some(host)) => Ok(format!("{}@{}", name, host)),
		}
	}
}
#[derive(Clone)]
pub struct EmojiService {
	db: DataBase,
	host: String,
	parse_emoji_str_regexp: regex::Regex,
	cache: Arc<RwLock<MemoryCache<String, MiEmoji>>>,
}
impl std::fmt::Debug for EmojiService {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("EmojiService")
			.field("db", &self.db)
			.field("host", &self.host)
			.field("parse_emoji_str_regexp", &self.parse_emoji_str_regexp)
			.finish()
	}
}
const PARSE_EMOJI_STR_REGEXP: &'static str =
	"^([0-9A-Za-z_\\.-]+){1}((@(\\w+\\.)+?)+(\\w{2,})){0,1}?$";
impl EmojiService {
	pub fn new(db: DataBase, host: String) -> Self {
		let parse_emoji_str_regexp = regex::Regex::new(PARSE_EMOJI_STR_REGEXP).unwrap();
		let cache = Arc::new(RwLock::new(MemoryCache::new()));
		Self {
			db,
			host,
			parse_emoji_str_regexp,
			cache,
		}
	}
	/**
	 * 複数の添付用(リモート)カスタム絵文字URLを解決する (キャシュ付き, 存在しないものは結果から除外される)
	 */
	pub async fn populate_emojis(
		&self,
		emojis: Vec<String>,
		user_host: Option<String>,
	) -> HashMap<String, String> {
		let job = emojis
			.iter()
			.map(|emoji| self.populate_emoji(emoji.clone(), user_host.clone()));
		let urls = futures::future::join_all(job).await;
		let mut map = HashMap::new();
		for (emoji, url) in emojis.into_iter().zip(urls) {
			if let Some(url) = url {
				map.insert(emoji, url);
			}
		}
		map
	}
	/**
	 * 添付用(リモート)カスタム絵文字URLを解決する
	 * @param emoji ノートやユーザープロフィールに添付された、またはリアクションのカスタム絵文字名 (:は含めない, リアクションでローカルホストの場合は@.を付ける (これはdecodeReactionで可能))
	 * @param user_host ノートやユーザープロフィールの所有者のホスト
	 * @returns URL文字列。 Noneは未マッチを意味する
	 */
	pub async fn populate_emoji(&self, emoji: String, user_host: Option<String>) -> Option<String> {
		let parsed = self.parse_emoji_str(&emoji, user_host);
		let name = parsed.name.as_ref()?;
		let host = parsed.host.as_ref()?;
		let key: String = (&parsed).try_into().ok()?;
		let emoji = {
			let rl = self.cache.read().await;
			rl.get(&key).cloned()
		};
		let emoji = match emoji {
			Some(cache_hit) => cache_hit,
			None => {
				let emoji: MiEmoji = {
					use crate::models::emoji::emoji::dsl::emoji;
					use crate::models::emoji::emoji::dsl::{host as dsl_host, name as dsl_name};
					use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
					use diesel_async::RunQueryDsl;
					emoji
						.filter(dsl_name.eq(&name))
						.filter(dsl_host.eq(&host))
						.select(MiEmoji::as_select())
						.first(
							&mut self
								.db
								.get_read_only()
								.await
								.map_err(|e| {
									eprintln!("{}:{} {:?}", file!(), line!(), e);
								})
								.ok()?,
						)
						.await
						.map_err(|e| {
							eprintln!("{}:{} {:?}", file!(), line!(), e);
						})
						.ok()
				}?;
				let mut wl = self.cache.write().await;
				wl.insert(key, emoji.clone(), Some(Duration::from_secs(5 * 60)));
				emoji
			}
		};
		if emoji.public_url.is_empty() {
			Some(emoji.original_url)
		} else {
			Some(emoji.public_url)
		}
	}
	pub fn normalize_reaction(&self, reaction: String) -> String {
		//reaction=":foo:"
		//reaction=":foo@example.com:"
		//reaction="🍮"
		if reaction.starts_with(":") && reaction.ends_with(":") {
			if reaction.contains("@") {
				let emoji = self.parse_emoji_str(&reaction[1..reaction.len() - 1], None);
				if let Ok(emoji) = TryInto::<String>::try_into(&emoji) {
					format!(":{}:", emoji)
				} else {
					reaction
				}
			} else {
				//hostが無い時はmatchする必要が無い
				format!(":{}@.:", &reaction[1..reaction.len() - 1])
			}
		} else {
			reaction
		}
	}
	pub fn parse_emoji_str(&self, emoji_name: &str, note_user_host: Option<String>) -> ParsedEmoji {
		let find = self.parse_emoji_str_regexp.find(emoji_name);
		match find {
			Some(m) => {
				let mut split = m.as_str().split("@");
				ParsedEmoji {
					name: split.next().map(|s| s.to_owned()),
					host: match split.next() {
						Some(s) => self.normalize_host(Some(s), note_user_host),
						None => note_user_host.as_ref().map(|s| to_puny_code(&s)),
					},
				}
			}
			None => ParsedEmoji {
				name: None,
				host: None,
			},
		}
	}
	pub fn normalize_host(
		&self,
		src: Option<&str>,
		note_user_host: Option<String>,
	) -> Option<String> {
		// クエリに使うホスト
		let host = match src {
			None => note_user_host, // ノートなどでホスト省略表記の場合はノートなどの所有者のホスト (ここがリアクションにマッチすることはない)
			Some(".") => None,      // .はローカルホスト (ここがマッチするのはリアクションのみ)
			Some(host) => {
				if self.host.as_str() == host {
					None // 自ホスト指定
				} else {
					Some(src.unwrap().to_owned()) // 指定されたホスト
				}
			}
		};
		host.map(|s| to_puny_code(&s))
	}
}
pub fn to_puny_code(s: &str) -> String {
	let mut parts = Vec::new();
	for s in s.split(".") {
		match punycode::encode(s) {
			Ok(mut v) => {
				if v.len() == s.len() + 1 && v.ends_with("-") && v.starts_with(s) {
					parts.push(Cow::Borrowed(s));
				} else {
					v.insert_str(0, "xn--");
					parts.push(Cow::Owned(v));
				}
			}
			Err(_) => parts.push(Cow::Borrowed(s)),
		}
	}
	parts.join(".")
}
#[test]
fn test_to_puny_code() {
	assert_eq!(punycode::encode("幼女"), Ok("vusz0j".to_owned()));
	assert_eq!(to_puny_code("幼女.art"), "xn--vusz0j.art".to_owned());
	assert_eq!(
		to_puny_code("exampleドメイン名例.jp"),
		"xn--example-er4fyliikhu827avim.jp".to_owned()
	);
	assert_eq!(
		to_puny_code("foo.ドメイン名例.jp"),
		"foo.xn--eckwd4c7cu47r2wf.jp".to_owned()
	);
	assert_eq!(
		to_puny_code("exampleドメイン名例foo.jp"),
		"xn--examplefoo-qx4ixo7jviu903bscp.jp".to_owned()
	);
	assert_eq!(to_puny_code("example.com"), "example.com".to_owned());
}
#[test]
fn test_parse_emoji_str_regexp() {
	let parse_emoji_str_regexp = regex::Regex::new(PARSE_EMOJI_STR_REGEXP).unwrap();
	assert_eq!(
		parse_emoji_str_regexp
			.find("foo")
			.map(|s| s.as_str().to_owned()),
		Some("foo".to_owned())
	);
	assert_eq!(
		parse_emoji_str_regexp
			.find("foo@example.com")
			.map(|s| s.as_str().to_owned()),
		Some("foo@example.com".to_owned())
	);
	assert_eq!(
		parse_emoji_str_regexp
			.find("yojoart@幼女.art")
			.map(|s| s.as_str().to_owned()),
		Some("yojoart@幼女.art".to_owned())
	);
	assert_eq!(
		parse_emoji_str_regexp
			.find("yojoart@m2.kzkr.xyz")
			.map(|s| s.as_str().to_owned()),
		Some("yojoart@m2.kzkr.xyz".to_owned())
	);
	assert_eq!(parse_emoji_str_regexp.find("yojoart@幼女"), None);
	assert_eq!(parse_emoji_str_regexp.find("無効@example.com"), None);
	assert_eq!(parse_emoji_str_regexp.find("無効"), None);
}
