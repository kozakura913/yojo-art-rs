use std::{io::Write, net::SocketAddr, sync::Arc};

use axum::{http::StatusCode, response::{IntoResponse, Response}, Router};
use diesel_async::AsyncPgConnection;
use redis::aio::MultiplexedConnection;
use service::{announcement::AnnouncementService, drive::DriveService, event::EventService, file_meta::FileMetaService, id_service::IdService, meta::MetaService, role::RoleService, user::UserService};
use s3::Bucket;
use serde::{Deserialize, Serialize};
mod browsersafe;
mod service;
mod models;
mod api;

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct ConfigFile{
	bind_addr: String,
	public_base_url:String,
	prefix:String,
	thumbnail_filter:FilterType,
	thumbnail_quality:f32,
	ffmpeg:Option<String>,
	ffmpeg_base_url:Option<String>,
	s3: S3Config,
	session_ttl: u64,
	part_max_size:u64,
	backend:String,
	full_upload_limit: u32,
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct MisskeyConfig{
	id: String,
	db: DBConfig,
	url: String,
	#[serde(rename = "proxyRemoteFiles")]
	proxy_remote_files:Option<bool>,
	#[serde(rename = "mediaProxy")]
	media_proxy:Option<String>,
	#[serde(rename = "remoteProxy")]
	remote_proxy:Option<String>,
	#[serde(rename = "apFileBaseUrl")]
	ap_file_base_url:Option<String>,
	redis:RedisConfig,
	#[serde(rename = "redisForPubsub")]
	redis_for_pubsub:Option<RedisConfig>,
}
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct S3Config{
	endpoint: String,
	bucket: String,
	region: String,
	access_key: String,
	secret_key: String,
	timeout:u64,
	path_style:bool,
}
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct RedisConfig{
	host: String,
	port: u16,
}
impl RedisConfig{
	fn to_url(&self)->String{
		let host=self.host.as_str();
		let port=self.port;
		format!("redis://{host}:{port}")
	}
}
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct DBConfig{
	host: String,
	port: u16,
	db: String,
	user:String,
	pass:String,
}
impl DBConfig{
	fn to_url(&self)->String{
		let user=self.user.as_str();
		let pass=self.pass.as_str();
		let host=self.host.as_str();
		let port=self.port;
		let db=self.db.as_str();
		format!("postgres://{user}:{pass}@{host}:{port}/{db}")
	}
}
#[derive(Clone,Debug)]
pub struct Context{
	bucket:Box<Bucket>,
	config:Arc<ConfigFile>,
	misskey_config:Arc<MisskeyConfig>,
	redis:MultiplexedConnection,
	client:reqwest::Client,
	role_service:RoleService,
	drive_service:DriveService,
	event_service:EventService,
	raw_db:DataBase,
	file_service: FileMetaService,
	user_service: UserService,
}
#[derive(Clone, Copy,Debug,Serialize,Deserialize)]
enum FilterType{
	Nearest,
	Triangle,
	CatmullRom,
	Gaussian,
	Lanczos3,
}
impl Into<image::imageops::FilterType> for FilterType{
	fn into(self) -> image::imageops::FilterType {
		match self {
			FilterType::Nearest => image::imageops::Nearest,
			FilterType::Triangle => image::imageops::Triangle,
			FilterType::CatmullRom => image::imageops::CatmullRom,
			FilterType::Gaussian => image::imageops::Gaussian,
			FilterType::Lanczos3 => image::imageops::Lanczos3,
		}
	}
}
impl Into<fast_image_resize::FilterType> for FilterType{
	fn into(self) -> fast_image_resize::FilterType {
		match self {
			FilterType::Nearest => fast_image_resize::FilterType::Box,
			FilterType::Triangle => fast_image_resize::FilterType::Bilinear,
			FilterType::CatmullRom => fast_image_resize::FilterType::CatmullRom,
			FilterType::Gaussian => fast_image_resize::FilterType::Mitchell,
			FilterType::Lanczos3 => fast_image_resize::FilterType::Lanczos3,
		}
	}
}
async fn shutdown_signal() {
	use tokio::signal;
	use futures::{future::FutureExt,pin_mut};
	let ctrl_c = async {
		signal::ctrl_c()
			.await
			.expect("failed to install Ctrl+C handler");
	}.fuse();

	#[cfg(unix)]
	let terminate = async {
		signal::unix::signal(signal::unix::SignalKind::terminate())
			.expect("failed to install signal handler")
			.recv()
			.await;
	}.fuse();
	#[cfg(not(unix))]
	let terminate = std::future::pending::<()>().fuse();
	pin_mut!(ctrl_c, terminate);
	futures::select!{
		_ = ctrl_c => {},
		_ = terminate => {},
	}
}
fn main() {
	let config_path=".config/config.json";
	if !std::path::Path::new(&config_path).exists(){
		let default_config=ConfigFile{
			bind_addr: "0.0.0.0:12200".to_owned(),
			public_base_url:"https://files.example.com/".to_owned(),
			prefix:"prefix".to_owned(),
			thumbnail_filter:FilterType::Lanczos3,
			thumbnail_quality:50f32,
			part_max_size:20*1024*1024,
			ffmpeg:Some("ffmpeg".to_owned()),
			ffmpeg_base_url:Some("https://files.example.com/".to_owned()),
			full_upload_limit:10*1024*1024,
			s3:S3Config{
				endpoint: "localhost:9000".to_owned(),
				region: "us-east-1".to_owned(),
				access_key: "example-user".to_owned(),
				secret_key: "example-password".to_owned(),
				bucket: "files".to_owned(),
				timeout: 5000,
				path_style: true,
			},
			session_ttl: 300,
			backend: "http://localhost:3000".to_owned(),
		};
		let default_config=serde_json::to_string_pretty(&default_config).unwrap();
		std::fs::File::create(&config_path).expect("create default config.json").write_all(default_config.as_bytes()).unwrap();
	}
	let misskey_config:MisskeyConfig=serde_yaml::from_reader(std::fs::File::open(&".config/default.yml").unwrap()).unwrap();
	let misskey_config=Arc::new(misskey_config);
	let file_service=FileMetaService::new();
	let config:ConfigFile=serde_json::from_reader(std::fs::File::open(&config_path).unwrap()).unwrap();
	let config=Arc::new(config);
	let bucket = s3::Bucket::new(
		&config.s3.bucket,
		s3::Region::Custom {
			region: config.s3.region.to_owned(),
			endpoint: config.s3.endpoint.to_owned(),
		},
		s3::creds::Credentials::new(Some(&config.s3.access_key),Some(&config.s3.secret_key),None,None,None).unwrap(),
	).unwrap();
	let bucket=if config.s3.path_style{
		bucket.with_path_style()
	}else{
		bucket
	};
	let redis=redis::Client::open(misskey_config.redis.to_url()).unwrap();
	let redis_for_pubsub=misskey_config.redis_for_pubsub.as_ref().map(|redis_for_pubsub|redis::Client::open(redis_for_pubsub.to_url()).unwrap());
	let rt=tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
	rt.block_on(async{
		let redis=redis.get_multiplexed_tokio_connection().await.unwrap();
		let redis_for_pubsub=match redis_for_pubsub{
			Some(redis_for_pubsub)=>redis_for_pubsub.get_multiplexed_tokio_connection().await.ok(),
			None=>None
		};
		let db=DataBase::open(&misskey_config.db.to_url()).await.unwrap();
		let id_service=IdService::new(&misskey_config);
		let meta_service=MetaService::new(db.clone());
		let role_service=RoleService::new(db.clone(),meta_service.clone());
		let announcement_service=AnnouncementService::new(db.clone());
		let user_service=UserService::new(redis.clone(),db.clone(),id_service.clone(),role_service.clone(),announcement_service);
		let event_service=EventService::new(redis_for_pubsub.clone().unwrap_or(redis.clone()),misskey_config.clone());
		let drive_service=DriveService::new(misskey_config.clone(),db.clone(),meta_service,role_service.clone(),id_service,user_service.clone(),event_service.clone());
		let client=reqwest::Client::new();
		let arg_tup=Context{
			bucket,
			config,
			redis,
			client,
			role_service,
			drive_service,
			event_service,
			file_service,
			raw_db:db,
			user_service,
			misskey_config,
		};
		let http_addr:SocketAddr = arg_tup.config.bind_addr.parse().unwrap();
		let app = Router::new();
		let app=api::route(&arg_tup,app);
		let listener = tokio::net::TcpListener::bind(&http_addr).await.unwrap();
		println!("server loaded");
		axum::serve(listener,app.into_make_service_with_connect_info::<SocketAddr>()).with_graceful_shutdown(shutdown_signal()).await.unwrap();
	});
}
#[derive(Debug,Serialize, Deserialize)]
pub struct UploadSession{
	user_id:String,
	s3_key: String,
	upload_id:Option<String>,
	content_type:String,
	part_etag:Vec<String>,
	part_number:Option<u32>,
	content_length:u64,
	md5_ctx_64:String,
	ext:Option<String>,
	comment: Option<String>,
	folder_id: Option<String>,
	is_sensitive: bool,
	force:bool,
	name: String,
	sensitive_threshold: f32,
	skip_sensitive_detection: bool,
}
pub(crate) fn md5_ontext_into_raw(ctx:md5::Context)->String{
	let ptr=Box::leak(Box::new(ctx));
	let s=unsafe{
		std::slice::from_raw_parts(ptr as *const _ as *const u8, std::mem::size_of::<md5::Context>())
	};
	use base64::Engine;
	let s=base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(s);
	unsafe{
		let _=Box::from_raw(ptr);
	}
	s
}
pub(crate) fn md5_ontext_from_raw(s:&String)->md5::Context{
	use base64::Engine;
	let raw=base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s).unwrap();
	let s = unsafe {
		Box::from_raw(raw.leak() as * mut _ as *mut md5::Context)
	};
	*s
}
impl Context{
	pub async fn upload_session(&mut self,authorization: Option<&axum::http::HeaderValue>,del:bool)->Result<(UploadSession,String),Response>{
		let session=match authorization.map(|v|v.to_str().map(|s|{
			if s.starts_with("Bearer "){
				Some(&s["Bearer ".len()..])
			}else{
				None
			}
		})){
			Some(Ok(Some(session_id)))=>{
				let sid={
					use sha2::{Sha256, Digest};
					let mut hasher = Sha256::new();
					hasher.update(session_id.as_bytes());
					let hash=hasher.finalize();
					use base64::Engine;
					base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash)
				};
				use redis::AsyncCommands;
				let res=if del{
					self.redis.get_del::<&String,String>(&format!("multipartUpload:{}",sid)).await.map(|v|serde_json::from_str::<UploadSession>(&v))
				}else{
					self.redis.get::<&String,String>(&format!("multipartUpload:{}",sid)).await.map(|v|serde_json::from_str::<UploadSession>(&v))
				};
				match res{
					Ok(Ok(s))=>Ok((s,sid)),
					Ok(Err(_))=>{
						return Err((StatusCode::INTERNAL_SERVER_ERROR).into_response())
					},
					_=>{
						return Err((StatusCode::FORBIDDEN).into_response())
					},
				}
			},
			e=>{
				eprintln!("{}:{} {:?}",file!(),line!(),e);
				return Err((StatusCode::BAD_REQUEST).into_response())
			}
		};
		session
	}
}
#[derive(Clone,Debug)]
pub struct DataBase(diesel_async::pooled_connection::bb8::Pool<AsyncPgConnection>);
pub type DBConnection<'a>=diesel_async::pooled_connection::bb8::PooledConnection<'a, AsyncPgConnection>;
impl DataBase{
	pub async fn open(database_url:&str)->Result<Self,String>{
		let config = diesel_async::pooled_connection::AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
		let pool = match diesel_async::pooled_connection::bb8::Pool::builder().build(config).await{
			Ok(p) => p,
			Err(e) => return Err(e.to_string()),
		};
		Ok(Self(pool))
	}
	pub async fn get(&self)->Option<DBConnection>{
		match self.0.get().await{
			Ok(c)=>Some(c),
			Err(e)=>{
				eprintln!("DB Error {:?}",e);
				None
			}
		}
	}
}
