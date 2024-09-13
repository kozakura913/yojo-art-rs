use diesel::{deserialize::{FromSql, FromSqlRow}, expression::AsExpression, serialize::ToSql, sql_types::Jsonb, Selectable};
use serde::{Deserialize, Serialize};

diesel::table! {
	#[sql_name = "drive_file"]
	drive_file (id) {
		id -> VarChar,
		userId -> Nullable<VarChar>,
		userHost -> Nullable<VarChar>,
		md5 -> VarChar,
		name -> VarChar,
		r#type -> VarChar,
		size -> Int4,
		size_long -> Int8,
		isLink -> Bool,
		isSensitive -> Bool,
		folderId -> Nullable<VarChar>,
		comment -> Nullable<VarChar>,
		blurhash -> Nullable<VarChar>,
		properties -> Jsonb,
		storedInternal -> Bool,
		url -> VarChar,
		thumbnailUrl -> Nullable<VarChar>,
		webpublicUrl -> Nullable<VarChar>,
		webpublicType -> Nullable<VarChar>,
		accessKey -> Nullable<VarChar>,
		thumbnailAccessKey -> Nullable<VarChar>,
		webpublicAccessKey -> Nullable<VarChar>,
		uri -> Nullable<VarChar>,
		src -> Nullable<VarChar>,
		maybeSensitive -> Bool,
		maybePorn -> Bool,
		requestHeaders -> Nullable<Jsonb>,
		requestIp -> Nullable<VarChar>,
	}
}
#[derive(PartialEq, Eq,Debug,Clone,diesel::Insertable,diesel::Queryable,Selectable,diesel::QueryableByName)]
#[diesel(table_name = drive_file)]
pub struct MiDriveFile{
	pub id:String,
	#[diesel(column_name = "userId")]
	pub user_id:Option<String>,
	#[diesel(column_name = "userHost")]
	pub user_host:Option<String>,
	pub md5:String,
	pub name:String,
	#[diesel(column_name = "type")]
	pub mime_type:String,
	pub size:i32,
	pub size_long:i64,
	/**
	 * 外部の(信頼されていない)URLへの直リンクか否か
	 */
	#[diesel(column_name = "isLink")]
	pub is_link:bool,
	#[diesel(column_name = "isSensitive")]
	pub is_sensitive:bool,
	#[diesel(column_name = "folderId")]
	pub folder_id:Option<String>,
	pub comment:Option<String>,
	pub blurhash:Option<String>,
	pub properties:FileProperties,
	#[diesel(column_name = "storedInternal")]
	pub stored_internal:bool,
	pub url:String,
	#[diesel(column_name = "thumbnailUrl")]
	pub thumbnail_url:Option<String>,
	#[diesel(column_name = "webpublicUrl")]
	pub webpublic_url:Option<String>,
	#[diesel(column_name = "webpublicType")]
	pub webpublic_type:Option<String>,
	#[diesel(column_name = "accessKey")]
	pub access_key:Option<String>,
	#[diesel(column_name = "thumbnailAccessKey")]
	pub thumbnail_access_key:Option<String>,
	#[diesel(column_name = "webpublicAccessKey")]
	pub webpublic_access_key:Option<String>,
	pub uri:Option<String>,
	pub src:Option<String>,
	#[diesel(column_name = "maybeSensitive")]
	pub maybe_sensitive:bool,
	#[diesel(column_name = "maybePorn")]
	pub maybe_porn:bool,
	/**Map<String,String> のはず*/
	#[diesel(column_name = "requestHeaders")]
	pub request_headers:Option<serde_json::Value>,
	#[diesel(column_name = "requestIp")]
	pub request_ip:Option<String>,
}
#[derive(PartialEq, Eq,Clone,Default,Debug,Serialize,Deserialize,FromSqlRow, AsExpression)]
#[diesel(sql_type = Jsonb)]
pub struct FileProperties{
	pub width: Option<u32>,
	pub height: Option<u32>,
	pub orientation:Option<i32>,
	#[serde(rename = "avgColor")]
	pub avg_color:Option<String>,
}
impl ToSql<Jsonb, diesel::pg::Pg> for FileProperties where serde_json::Value: ToSql<Jsonb, diesel::pg::Pg>{
	fn to_sql<'b>(&'b self,out:&mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result{
		<serde_json::Value as ToSql<Jsonb, diesel::pg::Pg>>::to_sql(&(serde_json::to_value(&self).map_err(|e|Box::new(e))?), &mut out.reborrow())
	}
}
impl<DB: diesel::backend::Backend> FromSql<Jsonb, DB> for FileProperties where serde_json::Value: FromSql<Jsonb, DB>{
	fn from_sql(bytes: DB::RawValue<'_>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let v=<serde_json::Value as FromSql<Jsonb, DB>>::from_sql(bytes)?;
		Ok(serde_json::from_str::<Self>(&v.to_string()).map_err(|e|Box::new(e))?)
	}
}
