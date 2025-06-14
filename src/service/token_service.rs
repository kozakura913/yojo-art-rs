use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::{
	DBConnection, DataBase, ServerError,
	models::{access_token::MiAccessToken, user::MiUser},
};

use super::id_service::IdService;

#[derive(Debug, Deserialize)]
pub struct Token(pub String);

#[derive(Clone, Debug)]
pub struct TokenService {
	db: DataBase,
	id_service: IdService,
}
pub enum TokenPermission {
	Token(MiAccessToken),
	Master(MiUser),
	None,
}
#[derive(Debug, Serialize, Deserialize)]
pub enum PermissionKind {
	#[serde(rename = "write:drive")]
	WriteDrive,
	#[serde(rename = "read:drive")]
	ReadDrive,
}
impl TokenPermission {
	pub fn is_allow(&self, key: PermissionKind) -> bool {
		let s = serde_json::to_string(&key).unwrap();
		match self {
			TokenPermission::Token(mi_access_token) => {
				mi_access_token.permission.binary_search(&s).is_ok()
			}
			TokenPermission::Master(_mi_user) => true,
			TokenPermission::None => false,
		}
	}
	pub async fn load_user(&self, db: &mut DBConnection<'_>) -> Result<Cow<MiUser>, ServerError> {
		match self {
			TokenPermission::Token(mi_access_token) => {
				Ok(MiUser::load_by_id(db, &mi_access_token.user_id)
					.await
					.map(|t| Cow::Owned(t))?)
			}
			TokenPermission::Master(mi_user) => Ok(Cow::Borrowed(mi_user)),
			TokenPermission::None => Err("guest user".into()),
		}
	}
	pub async fn as_user_id(&self) -> Option<&String> {
		match self {
			TokenPermission::Token(mi_access_token) => Some(&mi_access_token.user_id),
			TokenPermission::Master(mi_user) => Some(&mi_user.id),
			TokenPermission::None => None,
		}
	}
	pub async fn into_user(self, db: &mut DBConnection<'_>) -> Result<MiUser, ServerError> {
		match self {
			TokenPermission::Token(mi_access_token) => {
				Ok(MiUser::load_by_id(db, &mi_access_token.user_id).await?)
			}
			TokenPermission::Master(mi_user) => Ok(mi_user),
			TokenPermission::None => Err("guest user".into()),
		}
	}
}
impl TokenService {
	pub fn new(db: DataBase, id_service: IdService) -> Self {
		Self { db, id_service }
	}
	pub async fn get_permission(&self, token: &Token) -> TokenPermission {
		let token_id = token.0.as_str();
		let mut con = match self.db.get_read_only().await {
			Ok(con) => con,
			Err(e) => {
				eprintln!("{}:{} {:?}", file!(), line!(), e);
				return TokenPermission::None;
			}
		};
		let token = MiAccessToken::load_by_id(&mut con, token_id).await;
		if let Some(token) = token {
			return TokenPermission::Token(token);
		}
		let user = MiUser::load_by_token(&mut con, token_id).await;
		match user {
			Ok(user) => TokenPermission::Master(user),
			_ => TokenPermission::None,
		}
	}
}
