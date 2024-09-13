use diesel::Selectable;

diesel::table! {
	#[sql_name = "blocking"]
	blocking (id) {
		id -> VarChar,
		blockeeId -> VarChar,
		blockerId -> VarChar,
	}
}
#[derive(PartialEq,Eq,Debug,Clone,diesel::Insertable,diesel::Queryable,Selectable,diesel::QueryableByName)]
#[diesel(table_name = blocking)]
pub struct MiBlocking{
	pub id:String,
	#[diesel(column_name = "blockeeId")]
	pub blockee_id:String,
	#[diesel(column_name = "blockerId")]
	pub blocker_id:String,
}
