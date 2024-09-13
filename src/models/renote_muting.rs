use diesel::Selectable;

diesel::table! {
	#[sql_name = "renote_muting"]
	renote_muting (id) {
		id -> VarChar,
		muteeId -> VarChar,
		muterId -> VarChar,
	}
}
#[derive(PartialEq,Eq,Debug,Clone,diesel::Insertable,diesel::Queryable,Selectable,diesel::QueryableByName)]
#[diesel(table_name = renote_muting)]
pub struct MiRenoteMuting{
	pub id:String,
	#[diesel(column_name = "muteeId")]
	pub mutee_id:String,
	#[diesel(column_name = "muterId")]
	pub muter_id:String,
}
