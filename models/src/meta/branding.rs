use diesel::{
	Selectable,
	deserialize::{FromSql, FromSqlRow},
	expression::AsExpression,
	serialize::ToSql,
	sql_types::VarChar,
};

diesel::table! {
	meta (id) {
		id -> VarChar,
		name -> Nullable<VarChar>,
		shortName -> Nullable<VarChar>,
		description -> Nullable<VarChar>,
		maintainerName -> Nullable<VarChar>,
		maintainerEmail -> Nullable<VarChar>,
		langs -> Array<VarChar>,
		pinnedUsers -> Array<VarChar>,
		themeColor -> Nullable<VarChar>,
		mascotImageUrl -> Nullable<VarChar>,
		bannerUrl -> Nullable<VarChar>,
		backgroundImageUrl -> Nullable<VarChar>,
		logoImageUrl -> Nullable<VarChar>,
		iconUrl -> Nullable<VarChar>,
		app192IconUrl -> Nullable<VarChar>,
		app512IconUrl -> Nullable<VarChar>,
		serverErrorImageUrl -> Nullable<VarChar>,
		notFoundImageUrl -> Nullable<VarChar>,
		infoImageUrl -> Nullable<VarChar>,
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
#[diesel(table_name = meta)]
pub struct MiMetaBranding {
	pub id: String,
	pub name: Option<String>,
	#[diesel(column_name = "shortName")]
	pub short_name: Option<String>,
	pub description: Option<String>,
	/**
	 * メンテナの名前
	 */
	#[diesel(column_name = "maintainerName")]
	pub maintainer_name: Option<String>,
	/**
	 * メンテナの連絡先
	 */
	#[diesel(column_name = "maintainerEmail")]
	pub maintainer_email: Option<String>,
	pub langs: Vec<String>,
	#[diesel(column_name = "pinnedUsers")]
	pub pinned_users: Vec<String>,
	#[diesel(column_name = "themeColor")]
	pub theme_color: Option<String>,
	#[diesel(column_name = "mascotImageUrl")]
	pub mascot_image_url: Option<String>,
	#[diesel(column_name = "bannerUrl")]
	pub banner_url: Option<String>,
	#[diesel(column_name = "backgroundImageUrl")]
	pub background_image_url: Option<String>,
	#[diesel(column_name = "logoImageUrl")]
	pub logo_image_url: Option<String>,
	#[diesel(column_name = "iconUrl")]
	pub icon_url: Option<String>,
	#[diesel(column_name = "app192IconUrl")]
	pub app192_icon_url: Option<String>,
	#[diesel(column_name = "app512IconUrl")]
	pub app512_icon_url: Option<String>,
	#[diesel(column_name = "serverErrorImageUrl")]
	pub server_error_image_url: Option<String>,
	#[diesel(column_name = "notFoundImageUrl")]
	pub not_found_image_url: Option<String>,
	#[diesel(column_name = "infoImageUrl")]
	pub info_image_url: Option<String>,
}
