use diesel_async::AsyncPgConnection;

pub mod access_token;
pub mod announcement;
pub mod announcement_read;
pub mod avatar_decoration;
pub mod blocking;
pub mod common;
pub mod drive_file;
pub mod drive_folder;
pub mod emoji;
pub mod event;
pub mod follow_request;
pub mod following;
pub mod instance;
pub mod meta;
pub mod muting;
pub mod note;
pub mod note_reaction;
pub mod poll;
pub mod poll_vote;
pub mod renote_muting;
pub mod role;
pub mod user;
pub mod user_keypair;
pub mod user_memo;
pub mod user_note_pining;
pub mod user_profile;

pub use diesel::result::Error;
pub type DBConnection<'a> =
	diesel_async::pooled_connection::bb8::PooledConnection<'a, diesel_async::AsyncPgConnection>;

#[derive(Clone, Debug)]
pub struct DataBase(diesel_async::pooled_connection::bb8::Pool<AsyncPgConnection>);

impl DataBase {
	pub async fn open(database_url: &str) -> Result<Self, String> {
		let config = diesel_async::pooled_connection::AsyncDieselConnectionManager::<
			AsyncPgConnection,
		>::new(database_url);
		let pool = match diesel_async::pooled_connection::bb8::Pool::builder()
			.build(config)
			.await
		{
			Ok(p) => p,
			Err(e) => return Err(e.to_string()),
		};
		Ok(Self(pool))
	}
	pub async fn get_writeable(
		&self,
	) -> Result<DBConnection, diesel_async::pooled_connection::bb8::RunError> {
		self.0.get().await
	}
	pub async fn get_read_only(
		&self,
	) -> Result<DBConnection, diesel_async::pooled_connection::bb8::RunError> {
		self.0.get().await
	}
}
