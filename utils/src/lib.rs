extern crate proc_macro;
use quote::quote;
use syn::{ItemEnum, ItemStruct, parse_macro_input};

#[proc_macro_derive(PgJson)]
pub fn derive_pg_json(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let item = parse_macro_input!(input as ItemStruct);
	let struct_name = item.ident;
	quote! {
		impl ToSql<Jsonb, diesel::pg::Pg> for #struct_name
		where
			serde_json::Value: ToSql<Jsonb, diesel::pg::Pg>,
		{
			fn to_sql<'b>(
				&'b self,
				out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
			) -> diesel::serialize::Result {
				<serde_json::Value as ToSql<Jsonb, diesel::pg::Pg>>::to_sql(
					&(serde_json::to_value(&self).map_err(|e| Box::new(e))?),
					&mut out.reborrow(),
				)
			}
		}
		impl<DB: diesel::backend::Backend> FromSql<Jsonb, DB> for #struct_name
		where
			serde_json::Value: FromSql<Jsonb, DB>,
		{
			fn from_sql(bytes: DB::RawValue<'_>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
				let v = <serde_json::Value as FromSql<Jsonb, DB>>::from_sql(bytes)?;
				Ok(serde_json::from_str::<Self>(&v.to_string()).map_err(|e| Box::new(e))?)
			}
		}
	}
	.into()
}

/** EnumString用 */
#[proc_macro_derive(PgString)]
pub fn derive_pg_string(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let item = parse_macro_input!(input as ItemEnum);
	let struct_name = item.ident;
	quote! {
		impl ToSql<VarChar, diesel::pg::Pg> for #struct_name
		where
			String: ToSql<VarChar, diesel::pg::Pg>,
		{
			fn to_sql<'b>(
				&'b self,
				out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
			) -> diesel::serialize::Result {
				<String as ToSql<VarChar, diesel::pg::Pg>>::to_sql(&self.to_string(), &mut out.reborrow())
			}
		}
		impl<DB: diesel::backend::Backend> FromSql<VarChar, DB> for #struct_name
		where
			String: FromSql<VarChar, DB>,
		{
			fn from_sql(bytes: DB::RawValue<'_>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
				let v = <String as FromSql<VarChar, DB>>::from_sql(bytes)?;
				use std::str::FromStr;
				Ok(Self::from_str(&v).or_else(|e| Err(Box::new(e)))?)
			}
		}
	}
	.into()
}

/** EnumString用 */
#[proc_macro_derive(PgEnum, attributes(pg_type))]
pub fn derive_pg_enum(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let item = parse_macro_input!(input as ItemEnum);
	let struct_name = item.ident;
	let sql_type = item
		.attrs
		.iter()
		.filter(|attr| attr.path.is_ident("pg_type"))
		.find_map(|attr| {
			let name_val: syn::MetaNameValue = attr.parse_args().expect(&format!("{:?}", attr));
			if name_val.path.is_ident("sql_type") {
				Some(name_val)
			} else {
				None
			}
		})
		.expect("sql_type");
	let s = match sql_type.lit {
		syn::Lit::Str(s) => s,
		_ => panic!("sql_type.lit"),
	};
	let sql_type_ident = syn::Ident::new(&s.value(), struct_name.span());
	quote! {

        impl ToSql<#sql_type_ident, diesel::pg::Pg> for #struct_name
        {
            fn to_sql<'b>(
                &'b self,
                out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
            ) -> diesel::serialize::Result {
                use std::io::Write;
                out.write_all(&self.to_string().as_bytes()[..])?;
                use diesel::serialize::IsNull;
                Ok(IsNull::No)
            }
        }
        impl FromSql<#sql_type_ident, diesel::pg::Pg> for #struct_name
        {
            fn from_sql(bytes: diesel::pg::PgValue<'_>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
                let v=String::from_utf8(bytes.as_bytes().to_vec()).map_err(|e| Box::new(e))?;
                use std::str::FromStr;
                Ok(Self::from_str(&v).or_else(|e| Err(Box::new(e)))?)
            }
        }
    }.into()
}
