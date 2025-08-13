use diesel::Selectable;

use crate::DBConnection;

diesel::table! {
	#[sql_name = "user_keypair"]
	user_keypair (userId) {
		userId -> VarChar,
		publicKey -> VarChar,
		privateKey -> VarChar,
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
#[diesel(table_name = user_keypair)]
pub struct MiUserKeypair {
	#[diesel(column_name = "userId")]
	pub user_id: String,
	#[diesel(column_name = "publicKey")]
	pub public_key: String,
	#[diesel(column_name = "privateKey")]
	pub private_key: String,
}
impl MiUserKeypair {
	pub async fn load_by_user(
		con: &mut DBConnection<'_>,
		user_id: &str,
	) -> Result<Self, diesel::result::Error> {
		use diesel::QueryDsl;
		use diesel::SelectableHelper;
		use diesel::ExpressionMethods;
		use diesel_async::RunQueryDsl;
		use self::user_keypair::dsl::user_keypair;
		use self::user_keypair::dsl::*;
		user_keypair.filter(userId.eq(user_id))
			.select(Self::as_select())
			.first(con)
			.await
	}
}
