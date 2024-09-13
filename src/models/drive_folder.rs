use diesel::Selectable;

diesel::table! {
	#[sql_name = "drive_folder"]
	drive_folder (id) {
		id -> VarChar,
		userId -> Nullable<VarChar>,
		parentId -> Nullable<VarChar>,
		name -> VarChar,
	}
}
#[derive(PartialEq,Eq,Debug,Clone,diesel::Insertable,diesel::Queryable,Selectable,diesel::QueryableByName)]
#[diesel(table_name = drive_folder)]
pub struct MiDriveFolder{
	pub id:String,
	#[diesel(column_name = "userId")]
	pub user_id:Option<String>,
	#[diesel(column_name = "parentId")]
	pub parent_id:Option<String>,
	pub name:String,
}
