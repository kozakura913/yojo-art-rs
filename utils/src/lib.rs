extern crate proc_macro;
use quote::quote;
use syn::{parse_macro_input, ItemEnum, ItemStruct};

# [proc_macro_derive(PgJson)]
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
    }.into()
}

/** EnumString用 */
# [proc_macro_derive(PgString)]
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
    }.into()
}
