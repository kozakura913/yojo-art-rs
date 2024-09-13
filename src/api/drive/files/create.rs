use core::str;
use std::io::Write;

use axum::{extract::Multipart, http::StatusCode, response::IntoResponse};

use crate::{models::{access_token::MiAccessToken, user::MiUser}, Context};

#[derive(Default,Debug)]
struct RequestParms{
	name:Option<String>,
	ext:Option<String>,
	i:Option<String>,
	comment:Option<String>,
	is_sensitive:bool,
	size:u64,
}

pub async fn post(
	ctx:Context,
	mut multipart: Multipart,
)->axum::response::Response{
	println!("full upload");
	let mut req=RequestParms::default();
	let mut file_data=None;
	let mut force=false;
	let mut folder_id=None;
	while let Some(field) = multipart.next_field().await.unwrap_or(None) {
		let name = field.name();
		if name.is_none(){
			continue;
		}
		let name = name.unwrap().to_string();
		let data=field.bytes().await;
		if data.is_err(){
			continue;
		}
		let data = data.unwrap();

		if &name=="name"{
			req.name=String::from_utf8(data.to_vec()).ok();
		}
		if &name=="ext"{
			req.ext=String::from_utf8(data.to_vec()).ok();
		}
		if &name=="folder_id"{
			folder_id=String::from_utf8(data.to_vec()).ok();
		}
		if &name=="comment"{
			req.comment=String::from_utf8(data.to_vec()).ok();
		}
		if &name=="i"{
			req.i=String::from_utf8(data.to_vec()).ok();
		}
		if &name=="i"{
			req.i=String::from_utf8(data.to_vec()).ok();
		}
		if &name=="isSensitive"{
			req.is_sensitive=match str::from_utf8(&data){
				Ok("true")=>true,
				Ok("false")=>false,
				_=>false,
			}
		}
		if &name=="force"{
			force=match str::from_utf8(&data){
				Ok("true")=>true,
				Ok("false")=>false,
				_=>false,
			}
		}
		if &name=="size"{
			req.size=match str::from_utf8(&data){
				Ok(s)=>u64::from_str_radix(s,10).unwrap_or_default(),
				_=>0,
			}
		}
		if &name=="file"{
			file_data=Some(data);
		}
	}
	if file_data.is_none(){
		let mut header=axum::http::header::HeaderMap::new();
		header.insert(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN,"*".parse().unwrap());
		header.insert("X-ErrorStatus","NO DATA".parse().unwrap());
		return (axum::http::StatusCode::BAD_REQUEST,header).into_response();
	}
	let file_data=file_data.unwrap();

	let mut content_type="";
	if let Some(kind)=infer::get(&file_data){
		content_type=kind.mime_type();
		req.ext=Some(format!(".{}",kind.extension()));
		//println!("known content_type:{}",content_type);
	}
	if req.ext.as_ref().map(|s|s.as_str()) == Some("") {
		req.ext=match content_type{
			"image/jpeg"=>Some(".jpg"),
			"image/png"=>Some(".png"),
			"image/webp"=>Some(".webp"),
			"image/avif"=>Some(".avif"),
			"image/apng"=>Some(".apng"),
			"image/vnd.mozilla.apng"=>Some(".apng"),
			_=>None,
		}.map(|s|s.to_owned());
	}
	if content_type == "image/apng"{
		content_type="image/png";
	}
	if !crate::browsersafe::FILE_TYPE_BROWSERSAFE.contains(&content_type){
		content_type = "application/octet-stream";
		req.ext = None;
	}
	//let offset_time=chrono::Utc::now();
	let mut user=None;
	let mut register_preflight_result=Err(crate::service::drive::RegisterPreflightError::InternalServerError);
	if let Some(token)=req.i.as_ref(){
		if let Some(mut con)=ctx.raw_db.get().await{
			let db_token=MiAccessToken::load_by_id(&mut con, &token).await;
			user=match db_token{
				Some(token)=>MiUser::load_by_id(&mut con,&token.user_id).await,
				None=>MiUser::load_by_token(&mut con,&token).await
			};
			if let Some(me)=user.as_ref(){
				println!("call register_preflight");
				register_preflight_result=ctx.drive_service.register_preflight(
					Some(&me),
					req.size as i64,
					req.name.as_deref().unwrap_or_default(),
					req.ext.as_deref(),
					false,
					folder_id.as_deref(),
				).await;
			}else{
				println!("not found MiUser");
			}
		}else{
			let mut header=axum::http::header::HeaderMap::new();
			header.insert("X-ErrorStatus","DB Pool".parse().unwrap());
			return (axum::http::StatusCode::INTERNAL_SERVER_ERROR,header).into_response();
		}
	}else{
		println!("No Token")
	}
	//println!("preflight{}ms",(chrono::Utc::now()-offset_time).num_milliseconds());
	if let Err(e)=register_preflight_result{
		let mut header=axum::http::header::HeaderMap::new();
		header.insert("X-ErrorStatus",format!("{:?}",e).parse().unwrap());
		return (axum::http::StatusCode::BAD_REQUEST,header).into_response();
	}
	let res=register_preflight_result.unwrap();
	//println!("PREFLIGHT {:?}",res);

	let s3_key=format!("{}/{}{}",ctx.config.prefix,uuid::Uuid::new_v4().to_string(),req.ext.as_ref().map(|s|s.as_str()).unwrap_or(""));
	let mut md5sum=md5::Context::new();
	let (md5sum,content_md5) =match md5sum.write_all(&file_data){
		Ok(_)=>{
			let md5sum=md5sum.compute().0;
			let content_md5=s3::command::ContentMd5::from(md5sum.as_ref());
			let md5sum=md5sum.iter().map(|n| format!("{:02x}", n)).collect::<String>();
			(md5sum,content_md5)
		},
		Err(_)=>(String::new(),s3::command::ContentMd5::None)
	};

	//let offset_time=chrono::Utc::now();
	let thumbnail_size=2048;
	let cache_control="max-age=31536000, immutable";
	let detected_name=percent_encoding::percent_encode(res.detected_name.as_bytes(), percent_encoding::NON_ALPHANUMERIC);
	let content_disposition=format!("inline; filename=\"{}\"",detected_name);
	
	let (raw_upload,(mut thumbnail_upload,mut info))=futures_util::join!(
		ctx.bucket.put_object_with_metadata(&s3_key,&file_data,&content_type,content_md5.clone(),cache_control,&content_disposition),
		async{
			let info=match image::load_from_memory(&file_data){
				Ok(img)=>ctx.file_service.metadata(
					img,
					res.sensitive_threshold,
					res.skip_sensitive_detection,
					thumbnail_size,
					ctx.config.thumbnail_quality,
					ctx.config.thumbnail_filter.into(),
				).await,
				_=>Default::default(),
			};
			let thumbnail_bin=info.thumbnail.as_ref();
			(match thumbnail_bin{
				Some(thumbnail_bin)=>{
					let thumbnail_key=format!("{}/thumbnail-{}{}",ctx.config.prefix,uuid::Uuid::new_v4().to_string(),".webp");
					match ctx.bucket.put_object_with_metadata(&thumbnail_key,&thumbnail_bin,"image/webp",s3::command::ContentMd5::Auto,cache_control,&content_disposition).await{
						Ok(_)=>Ok(Some(thumbnail_key)),
						Err(e)=>Err(e),
					}
				},
				None=>Ok(None)
			},info)
		}
	);
	if content_type.starts_with("video/"){
		info=ctx.file_service.ffmpeg_metadata(&ctx.config,&s3_key,thumbnail_size,res.sensitive_threshold,res.skip_sensitive_detection).await.unwrap_or_default();
		let thumbnail_bin=info.thumbnail.as_ref();
		thumbnail_upload=match thumbnail_bin{
			Some(thumbnail_bin)=>{
				let thumbnail_key=format!("{}/thumbnail-{}{}",ctx.config.prefix,uuid::Uuid::new_v4().to_string(),".webp");
				match ctx.bucket.put_object_with_metadata(&thumbnail_key,&thumbnail_bin,"image/webp",s3::command::ContentMd5::Auto,cache_control,&content_disposition).await{
					Ok(_)=>Ok(Some(thumbnail_key)),
					Err(e)=>Err(e),
				}
			},
			None=>Ok(None)
		};
	}
	//println!("name:{}",res.detected_name);
	//println!("md5sum:{}",md5sum);
	//println!("sensitive:{}",info.maybe_sensitive.unwrap_or_default());
	//println!("blurhash:{}",info.blurhash.clone().unwrap_or_default());
	//println!("metadata{}ms",(chrono::Utc::now()-offset_time).num_milliseconds());
	//println!("s3_key:{}",&s3_key);
	match raw_upload{
		Ok(_resp) => {},
		Err(e) =>{
			eprintln!("{}:{} {:?}",file!(),line!(),e);
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		},
	}
	let thumbnail_key=match thumbnail_upload{
		Ok(key) => {
			key
		},
		Err(e) =>{
			eprintln!("{}:{} {:?}",file!(),line!(),e);
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		},
	};
	let res=ctx.drive_service.register_file(
		user.as_ref(),
		s3_key.as_str(),
		folder_id.as_deref(),
		req.comment.as_deref(),
		info.blurhash.as_deref(),
		false,
		info.width,
		info.height,
		info.maybe_sensitive.unwrap_or_default(),
		"",
		req.is_sensitive,
		None,
		None,
		res.detected_name,
		md5sum,
		content_type.to_owned(),
		file_data.len() as i64,
		force,
		thumbnail_key.as_deref(),
		ctx.config.public_base_url.clone(),
	).await;
	if res.is_none(){
		return (axum::http::StatusCode::BAD_REQUEST).into_response();
	}
	let res=res.unwrap();
	//let (status,res)=res.unwrap();
	let mut header=axum::http::header::HeaderMap::new();
	header.insert(axum::http::header::CONTENT_TYPE,"application/json".parse().unwrap());
	let status=axum::http::StatusCode::from_u16(200).unwrap_or(axum::http::StatusCode::BAD_GATEWAY);
	(status,header,serde_json::to_string(&res.1.unwrap_or(serde_json::Value::Null)).unwrap_or_default()).into_response()
}
