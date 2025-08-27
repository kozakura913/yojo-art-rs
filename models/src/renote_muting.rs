use diesel::Selectable;

use crate::DBConnection;

diesel::table! {
	#[sql_name = "renote_muting"]
	renote_muting (id) {
		id -> VarChar,
		muteeId -> VarChar,
		muterId -> VarChar,
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
#[diesel(table_name = renote_muting)]
pub struct MiRenoteMuting {
	pub id: String,
	#[diesel(column_name = "muteeId")]
	pub mutee_id: String,
	#[diesel(column_name = "muterId")]
	pub muter_id: String,
}

pub async fn renote_muting(
	con: &mut DBConnection<'_>,
	me_id: &str,
) -> Result<Vec<String>, diesel::result::Error> {
	use self::renote_muting::dsl::renote_muting;
	use self::renote_muting::dsl::*;
	use diesel::{ExpressionMethods, QueryDsl};
	use diesel_async::RunQueryDsl;
	renote_muting
		.filter(muterId.eq(me_id))
		.select(muteeId)
		.load(con)
		.await
}
