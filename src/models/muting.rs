use chrono::NaiveDateTime;
use diesel::Selectable;

diesel::table! {
	#[sql_name = "muting"]
	muting (id) {
		id -> VarChar,
		expiresAt -> Nullable<Timestamp>,
		muteeId -> VarChar,
		muterId -> VarChar,
	}
}
#[derive(PartialEq,Eq,Debug,Clone,diesel::Insertable,diesel::Queryable,Selectable,diesel::QueryableByName)]
#[diesel(table_name = muting)]
pub struct MiMuting{
	pub id:String,
	#[diesel(column_name = "expiresAt")]
	pub expires_at:Option<NaiveDateTime>,
	#[diesel(column_name = "muteeId")]
	pub mutee_id:String,
	#[diesel(column_name = "muterId")]
	pub muter_id:String,
}
