use axum::{http::StatusCode, response::IntoResponse};
use futures::TryStreamExt;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;
use tokio_util::io::StreamReader;

use crate::{models::{access_token::MiAccessToken, user::MiUser}, Context, UploadSession};

#[derive(Debug, Deserialize)]
pub struct RequestParams{
	i: String,//トークン必須
	content_length:Option<u64>,
	#[serde(rename = "folderId")]
	folder_id:Option<String>,
	name:Option<String>,
	#[serde(rename = "isSensitive")]
	is_sensitive:bool,
	comment:Option<String>,
	force:bool,
}
#[derive(Debug, Serialize)]
pub struct ResponseBody{
	allow_upload:bool,
	min_split_size:u32,
	max_split_size:u64,
	session_id:String,
}
pub async fn post(
	mut ctx:Context,
	request: axum::extract::Request,
)->axum::response::Response{
	let min_size=5*1024*1024;//最小5MB
	let stream=request.into_body().into_data_stream();
	let body_with_io_error = stream.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err));
	let mut body_reader = StreamReader::new(body_with_io_error);
	let mut buf=vec![];
	if let Err(e)=body_reader.read_to_end(&mut buf).await{
		eprintln!("{}:{} {:?}",file!(),line!(),e);
		return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
	}
	let q=match serde_json::from_slice::<RequestParams>(&buf){
		Ok(v)=>v,
		Err(e)=>{
			eprintln!("{}:{} {:?}",file!(),line!(),e);
			return (StatusCode::BAD_REQUEST).into_response();
		}
	};
	//let offset_time=chrono::Utc::now();

	let user;
	let mut register_preflight_result=Err(crate::service::drive::RegisterPreflightError::InternalServerError);
	if let Some(mut con)=ctx.raw_db.get().await{
		let db_token=MiAccessToken::load_by_id(&mut con, &q.i).await;
		user=match db_token{
			Some(token)=>MiUser::load_by_id(&mut con,&token.user_id).await,
			None=>MiUser::load_by_token(&mut con,&q.i).await
		};
		if let Some(me)=user.as_ref(){
			println!("call register_preflight");
			register_preflight_result=ctx.drive_service.register_preflight(
				Some(&me),
				q.content_length.unwrap_or_default() as i64,
				q.name.as_deref().unwrap_or_default(),
				None,
				false,
				q.folder_id.as_deref(),
			).await;
		}else{
			println!("not found MiUser");
		}
	}else{
		let mut header=axum::http::header::HeaderMap::new();
		header.insert("X-ErrorStatus","DB Pool".parse().unwrap());
		return (axum::http::StatusCode::INTERNAL_SERVER_ERROR,header).into_response();
	}
	//println!("preflight{}ms",(chrono::Utc::now()-offset_time).num_milliseconds());
	if let Err(e)=register_preflight_result{
		let mut header=axum::http::header::HeaderMap::new();
		header.insert("X-ErrorStatus",format!("{:?}",e).parse().unwrap());
		return (axum::http::StatusCode::BAD_REQUEST,header).into_response();
	}
	let backend_res=register_preflight_result.unwrap();
	//println!("PREFLIGHT {:?}",res);
	println!("content_length:{:?}",q.content_length);
	let mut res=ResponseBody{
		allow_upload:true,
		min_split_size:min_size,
		max_split_size:ctx.config.part_max_size,
		session_id:uuid::Uuid::new_v4().to_string(),
	};
	let s3_key=format!("{}/{}",ctx.config.prefix,uuid::Uuid::new_v4().to_string());
	//進行中の分割アップロードの一覧が取れる。これを使って適当に掃除する
	//bucket.list_multiparts_uploads(Some("/"), Some("/"));
	let md5_ctx_64=crate::md5_ontext_into_raw(md5::Context::new());
	let session=UploadSession{
		user_id:user.as_ref().unwrap().id.clone(),
		s3_key,
		part_number:None,
		content_length:0,
		upload_id:None,
		content_type:"application/octet-stream".to_owned(),
		part_etag:vec![],
		md5_ctx_64,
		ext: None,
		comment:q.comment,
		folder_id:q.folder_id,
		is_sensitive:q.is_sensitive,
		name:backend_res.detected_name,
		force:q.force,
		sensitive_threshold:backend_res.sensitive_threshold,
		skip_sensitive_detection:backend_res.skip_sensitive_detection,
	};
	let session=serde_json::to_string(&session).unwrap();
	let sid={
		use sha2::{Sha256, Digest};
		let mut hasher = Sha256::new();
		hasher.update(res.session_id.as_bytes());
		let hash=hasher.finalize();
		use base64::Engine;
		base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash)
	};
	let mut header=axum::http::header::HeaderMap::new();
	header.insert(axum::http::header::CONTENT_TYPE,"application/json".parse().unwrap());
	if let Err(e)=ctx.redis.set_ex::<&String,String,()>(&format!("multipartUpload:{}",sid),session,ctx.config.session_ttl).await{
		eprintln!("{}:{} {:?}",file!(),line!(),e);
		res.allow_upload=false;
		(StatusCode::INTERNAL_SERVER_ERROR,header,serde_json::to_string(&res).unwrap()).into_response()
	}else{
		(StatusCode::OK,header,serde_json::to_string(&res).unwrap()).into_response()
	}
}
