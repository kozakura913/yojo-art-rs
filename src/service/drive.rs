use std::{borrow::Cow, str::FromStr, sync::Arc};

use crate::{models::{self, drive_file::{FileProperties, MiDriveFile}, drive_folder::MiDriveFolder, meta::SensitiveMediaDetection, user::MiUser, user_profile::MiUserProfile}, service::{self, event::{DriveEventType, MainEventType}}, DBConnection, DataBase, MisskeyConfig};

use super::{event::EventService, id_service::IdService, meta::MetaService, role::RoleService, user::UserService};

#[derive(Clone,Debug)]
pub struct RegisterPreflightResult{
	pub skip_sensitive_detection: bool,
	pub sensitive_threshold: f32,
	pub enable_sensitive_media_detection_for_videos: bool,
	pub detected_name: String,
}
#[derive(Clone,Debug)]
pub struct DriveService{
	config:Arc<MisskeyConfig>,
	db:DataBase,
	meta_service:MetaService,
	role_service:RoleService,
	id_service:IdService,
	user_service:UserService,
	event_service:EventService,
}
#[derive(Clone,Debug)]
pub enum RegisterPreflightError{
	InternalServerError,
	FileSizeLimitOver,
	ExtTooLarge,
	BadExt,
	NoFreeSpace,
	FolderNotFound,
}
impl DriveService{
	pub fn new(
		config:Arc<MisskeyConfig>,
		db:DataBase,
		meta_service:MetaService,
		role_service:RoleService,
		id_service:IdService,
		user_service:UserService,
		event_service:EventService,
	)->Self{
		Self{
			config,
			db,
			meta_service,
			role_service,
			id_service,
			user_service,
			event_service,
		}
	}
	pub async fn register_preflight(&self,
		user:Option<&MiUser>,//system user = None
		size:i64,
		name:&str,
		ext:Option<&str>,
		is_link:bool,
		folder_id:Option<&str>,
	)->Result<RegisterPreflightResult,RegisterPreflightError> {
		let mut skip_nsfw_check ;
		let instance = self.meta_service.load(true).await.ok_or(RegisterPreflightError::InternalServerError)?;
		let user_role_nsfw =self.role_service.get_user_policies(user.as_ref().map(|user|user.id.as_str())).await.always_mark_nsfw.unwrap_or_default();
		if user.is_none() {
			//システムユーザーが作るファイルにはセンシティブ検出を適用しない
			skip_nsfw_check = true;
		} else if user_role_nsfw {
			skip_nsfw_check = true;
		}else{
			skip_nsfw_check=false;
		}
		if instance.sensitive_media_detection == SensitiveMediaDetection::None {
			skip_nsfw_check = true;
		}
		if let Some(user)=user.as_ref() {
			if instance.sensitive_media_detection == SensitiveMediaDetection::Local && user.host.is_some(){
				skip_nsfw_check = true;
			}
			if instance.sensitive_media_detection == SensitiveMediaDetection::Remote && user.host.is_none(){
				skip_nsfw_check = true;
			}
		}
		//ファイル単位の容量制限チェック
		if let Some(user)=user.as_ref(){
			if user.host.is_some() {
				//remote user skip
			} else {
				let policies=self.role_service.get_user_policies(Some(user.id.as_str())).await;
				if size > policies.file_size_limit.unwrap().saturating_mul(1024 * 1024) {
					return Err(RegisterPreflightError::FileSizeLimitOver);
				}
			}
		}else{
			//system user skip
		}

		if let Some(ext) = ext {
			if ext.len() > 50 {
				return Err(RegisterPreflightError::ExtTooLarge);
			}
			if ext.find('.') == None || !validate_file_name(ext) {
				return Err(RegisterPreflightError::BadExt);
			}
		}
		let detected_name = correct_filename(
			// DriveFile.nameは256文字, validateFileNameは200文字制限であるため、
			// extを付加してデータベースの文字数制限に当たることはまずない
			if validate_file_name(name){
				name
			}else{
				"untitled"
			},
			ext,
		);
		//#region Check drive usage
		let mut con=self.db.get().await.ok_or(RegisterPreflightError::InternalServerError)?;
		if !is_link {
			if let Some(user)=user.as_ref(){
				let usage = calc_drive_usage_of(&mut con,&user.id).await;

				let policies = self.role_service.get_user_policies(Some(user.id.as_str())).await;
				let drive_capacity = 1024 * 1024 * policies.drive_capacity_mb.unwrap_or_default();

				// If usage limit exceeded
				if drive_capacity < usage + size {
				return Err(RegisterPreflightError::NoFreeSpace);
				}
			}
		}
		//#endregion
		let drive_folder=fetch_folder(&mut con,folder_id,user.as_ref().map(|u|u.id.as_str())).await;
		if folder_id.is_some()&&drive_folder.is_none(){
			return Err(RegisterPreflightError::FolderNotFound);
		}
		let sensitive_threshold=match instance.sensitive_media_detection_sensitivity{
			models::meta::SensitiveMediaDetectionSensitivity::VeryHigh => 0.1,
			models::meta::SensitiveMediaDetectionSensitivity::High => 0.3,
			models::meta::SensitiveMediaDetectionSensitivity::Medium => 0.5,
			models::meta::SensitiveMediaDetectionSensitivity::Low => 0.7,
			models::meta::SensitiveMediaDetectionSensitivity::VeryLow => 0.9,
		};
		Ok(RegisterPreflightResult{
			skip_sensitive_detection: skip_nsfw_check,
			sensitive_threshold,
			enable_sensitive_media_detection_for_videos: instance.enable_sensitive_media_detection_for_videos,
			detected_name,
		})
	}
	pub async fn register_file(&self,
		user:Option<&MiUser>,
		access_key:&str,
		folder_id:Option<&str>,
		comment:Option<&str>,
		blurhash:Option<&str>,
		is_link:bool,
		width:u32,
		height:u32,
		maybe_sensitive:bool,
		request_ip:&str,
		sensitive:bool,
		url:Option<&str>,
		uri:Option<&str>,
		detected_name:String,
		md5:String,
		content_type:String,
		size:i64,
		force:bool,
		thumbnail_key:Option<&str>,
		base_url:String,
	)->Option<(MiDriveFile,Option<serde_json::Value>)>{
		let mut con=self.db.get().await?;
		let user_id=user.as_ref().map(|user|user.id.as_str());
		let instance=self.meta_service.load(true).await?;
		let (user_role_nsfw,profile) = match user_id{
			Some(user_id)=>(self.role_service.get_user_policies(Some(user_id)).await.always_mark_nsfw.unwrap(),MiUserProfile::load_by_user(&mut con, user_id).await),
			None=>(service::role::DEFAULT_POLICIES.always_mark_nsfw.unwrap(),None),
		};

		let folder = fetch_folder(&mut con,folder_id,user_id).await;

		let mut properties=FileProperties::default();

		if width!=0 {
			properties.width = Some(width);
		}
		if height!=0 {
			properties.height = Some(height);
		}
		if user_id.is_some() && !force {
			// Check if there is a file with the same hash
			use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
			use diesel_async::RunQueryDsl;
			let user_id=user_id.unwrap();
			let target_hash=md5.as_str();
			let matched :Option<MiDriveFile>={
				use crate::models::drive_file::drive_file::dsl::drive_file;
				use crate::models::drive_file::drive_file::dsl::*;
				drive_file.filter(userId.eq(user_id)).filter(md5.eq(target_hash)).select(MiDriveFile::as_select()).first(&mut con).await.map_err(|e|{
					eprintln!("{:?}",e);
				})
			}.ok();

			if let Some(mut matched)=matched {
				println!("file with same hash is found: {}",matched.id);
				if sensitive && !matched.is_sensitive {
					// The file is federated as sensitive for this time, but was federated as non-sensitive before.
					// Therefore, update the file to sensitive.
					use crate::models::drive_file::drive_file::dsl::drive_file;
					use crate::models::drive_file::drive_file::dsl::*;
					let is_ok=diesel::update(drive_file.filter(id.eq(matched.id.as_str()))).set(isSensitive.eq(true)).execute(&mut con).await.map_err(|e|{
						eprintln!("{:?}",e);
					}).is_ok();
					if is_ok{
						matched.is_sensitive = true;
					}
				}
				let packed_file=self.pack(&mut con,&matched,true,false,false,folder.as_ref(),user).await;
				return Some((matched,packed_file));
			}
		}

		let mut file = MiDriveFile{
			id : self.id_service.gen(None),
			user_id : user.as_ref().map(|user|user.id.to_owned()),
			user_host : user.as_ref().map(|user|user.host.to_owned()).unwrap_or_default(),
			folder_id : folder.as_ref().map(|folder|folder.id.to_owned()),
			comment : comment.map(|comment|comment.to_owned()),
			properties,
			blurhash : blurhash.map(|blurhash|blurhash.to_owned()),
			is_link,
			request_ip : None,
			request_headers : None,
			maybe_sensitive,
			maybe_porn : false,
			is_sensitive : sensitive,
			md5,
			size : size.min(i32::MAX.into()).try_into().unwrap(),
			size_long : size,
			name:detected_name,
			mime_type:content_type,
			stored_internal:false,
			src: url.map(|s|s.to_owned()),
			uri: uri.map(|s|s.to_owned()),
			url: "".to_owned(),//TODO オブジェクトストレージの公開URL
			thumbnail_url: None,//TODO
			webpublic_url: None,//TODO
			webpublic_type: None,//TODO
			access_key: None,//TODO
			thumbnail_access_key: None,//TODO
			webpublic_access_key: None,//TODO
		};

		if let Some(profile)=profile.as_ref(){
			if profile.always_mark_nsfw{
				file.is_sensitive = true;
			}
		}
		if let Some(user)=user.as_ref(){
			if is_media_silenced_host(&instance.media_silenced_hosts, user.host.as_deref()){
				file.is_sensitive = true;
			}
		}
		if maybe_sensitive && profile.as_ref().map(|profile|profile.auto_sensitive).unwrap_or_default(){
			file.is_sensitive = true;
		}
		if maybe_sensitive && instance.set_sensitive_flag_automatically{
			file.is_sensitive = true;
		}
		if user_role_nsfw{
			file.is_sensitive = true;
		}

		if is_link{
			if let Some(url) = url {
				file.url = url.to_string();
				// ローカルプロキシ用
				file.access_key = Some(uuid::Uuid::new_v4().to_string());
				file.thumbnail_access_key = Some(format!("thumbnail-{}",uuid::Uuid::new_v4()));
				file.webpublic_access_key = Some(format!("webpublic-{}",uuid::Uuid::new_v4()));
			}
			use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
			use diesel_async::RunQueryDsl;
			let db_res={
				use crate::models::drive_file::drive_file::dsl::drive_file;
				diesel::insert_into(drive_file).values(&file).execute(&mut con).await
			};
			if let Err(e)=db_res{
				eprintln!("drive File Insert Error {:?}",e);
				file={
					use crate::models::drive_file::drive_file::dsl::drive_file;
					use crate::models::drive_file::drive_file::dsl::*;
					drive_file.filter(userId.eq(user_id)).filter(url.eq(file.url.as_str())).select(MiDriveFile::as_select()).first(&mut con).await.map_err(|e|{
						eprintln!("{:?}",e);
					})
				}.ok()?;
			}
		} else {
			//const isRemote = user ? this.userEntityService.isRemoteUser(user) : false;
			//file = await (this.save(file, path, detectedName, contentType, md5, size, isRemote));
			file.url = format!("{}{}",base_url,access_key);
			if let Some(thumbnail_key)=thumbnail_key{
				file.thumbnail_url = Some(format!("{}{}",base_url,thumbnail_key));
				file.thumbnail_access_key = Some(thumbnail_key.to_owned());
			}
			file.webpublic_url = None;
			file.access_key = Some(access_key.to_owned());
			file.webpublic_access_key = None;
			file.webpublic_type = None;
			file.stored_internal = false;

			use diesel_async::RunQueryDsl;
			let db_res={
				use crate::models::drive_file::drive_file::dsl::drive_file;
				diesel::insert_into(drive_file).values(&file).execute(&mut con).await.map_err(|e|{
					eprintln!("{:?}",e);
					e
				})
			};
		}
		println!("drive file has been created {}",file.id);

		let packed_file=self.pack(&mut con,&file,true,false,false,folder.as_ref(),user).await;
		if let Some(user)=user.as_ref() {
			if let Some(packed_file)=packed_file.as_ref(){
				// Publish driveFileCreated event
				let _=self.event_service.publish_main_stream(&user.id, Some(MainEventType::DriveFileCreated),Some(packed_file.clone())).await;
				let _=self.event_service.publish_drive_stream(&user.id,Some(DriveEventType::FileCreated), Some(packed_file.clone())).await;
			};
		}

		//this.driveChart.update(file, true);
		if file.user_host.is_none() {
			// ローカルユーザーのみ
			//this.perUserDriveChart.update(file, true);
		} else {
			if self.meta_service.load(true).await.unwrap().enable_charts_for_federated_instances {
				//this.instanceChart.updateDrive(file, true);
			}
		}
		Some((file,packed_file))
	}
	pub async fn pack(
		&self,
		con:&mut DBConnection<'_>,
		file: &MiDriveFile,
		is_my_file:bool,//default=false
		detail:bool,//default=false
		with_user:bool,//default=false
		folder:Option<&MiDriveFolder>,
		user:Option<&MiUser>,
	)-> Option<serde_json::Value>{
		let mut map=serde_json::Map::new();
		map.insert("id".into(),file.id.as_str().into());
		map.insert("createdAt".into(),format!("{}",self.id_service.parse(file.id.as_ref())?.format("%+")).into());
		map.insert("name".into(),file.name.as_str().into());
		map.insert("type".into(),file.mime_type.as_str().into());
		map.insert("md5".into(),file.md5.as_str().into());
		map.insert("size".into(),file.size_long.max(file.size as i64).into());
		map.insert("isSensitive".into(),file.is_sensitive.into());
		map.insert("blurhash".into(),file.blurhash.as_ref().map(|c|serde_json::Value::String(c.to_string())).unwrap_or(serde_json::Value::Null));
		if is_my_file{
			map.insert("properties".into(),serde_json::to_value(file.properties.clone()).ok()?);
		}else{
			map.insert("properties".into(),serde_json::to_value(get_public_properties(file)).ok()?);
		}
		if is_my_file{
			map.insert("url".into(),file.url.as_str().into());
		}else{
			map.insert("url".into(),self.get_public_url(file,None,false).into());
		}
		map.insert("thumbnailUrl".into(),self.get_thumbnail_url(file).into());
		map.insert("comment".into(),file.comment.as_ref().map(|c|serde_json::Value::String(c.to_string())).unwrap_or(serde_json::Value::Null));
		map.insert("folderId".into(),file.folder_id.as_ref().map(|id|serde_json::Value::String(id.to_string())).unwrap_or(serde_json::Value::Null));
		if detail{
			if let Some(folder_id)=file.folder_id.as_ref(){
				let folder=if let Some(folder)=folder{
					Some(Cow::Borrowed(folder))
				}else{
					fetch_folder(con, Some(folder_id), file.user_id.as_deref()).await.map(|v|Cow::Owned(v))
				};
				map.insert("folder".into(),self.pack_folder(con,folder.as_ref().map(|v| &**v),true).await.unwrap_or(serde_json::Value::Null));
			}else{
				map.insert("folder".into(),serde_json::Value::Null);
			}
		}else{
			map.insert("folder".into(),serde_json::Value::Null);
		}
		if with_user{
			map.insert("userId".into(),file.user_id.as_ref().map(|c|serde_json::Value::String(c.to_string())).unwrap_or(serde_json::Value::Null));
			let pack_user=if let Some(user_id)=file.user_id.as_ref(){
				let user=if let Some(user)=user{
					Some(Cow::Borrowed(user))
				}else{
					MiUser::load_by_id(con,user_id).await.map(|v|Cow::Owned(v))
				};
				match user.as_ref(){
					Some(user)=>{
						self.user_service.pack(user,Some(user.id.as_ref()),&Default::default()).await
					},
					None=>None
				}
			}else{
				None
			};
			map.insert("user".into(),pack_user.unwrap_or(serde_json::Value::Null));
		}else{
			map.insert("userId".into(),serde_json::Value::Null);
			map.insert("user".into(),serde_json::Value::Null);
		}
		Some(serde_json::Value::Object(map))
	}
	async fn pack_folder(
		&self,
		con:&mut DBConnection<'_>,
		folder: Option<&MiDriveFolder>,
		detail: bool,
	)->Option<serde_json::Value> {
		let mut map=self.pack_folder0(con,folder,detail).await?;
		if detail{
			let folder=folder?;
			let mut parent_id=folder.parent_id.clone();
			let mut parent_stack=Vec::new();
			loop{
				if let Some(folder_id)=parent_id{
					let parent=fetch_folder(con, Some(&folder_id), folder.user_id.as_ref().map(|x| x.as_str())).await;
					parent_id=parent.as_ref().map(|v|v.parent_id.clone()).unwrap_or_default();
					if let Some(v)=self.pack_folder0(con,parent.as_ref(),true).await{
						parent_stack.push(v);
					}else{
						break;
					}
				}else{
					break;
				}
			}
			fn insert(map:&mut serde_json::Map<String, serde_json::Value>,mut arr:Vec<serde_json::Map<String, serde_json::Value>>){
				if arr.is_empty(){
					return;
				}
				let mut v=arr.remove(0);
				insert(&mut v,arr);
				map.insert("parent".into(),serde_json::Value::Object(v));
			}
			insert(&mut map,parent_stack);
		}
		Some(serde_json::Value::Object(map))
	}
	async fn pack_folder0(
		&self,
		con:&mut DBConnection<'_>,
		folder: Option<&MiDriveFolder>,
		detail: bool,
	)->Option<serde_json::Map<String,serde_json::Value>> {
		let folder=folder?;
		let mut map=serde_json::Map::new();
		map.insert("id".into(),folder.id.as_str().into());
		map.insert("createdAt".into(),format!("{}",self.id_service.parse(folder.id.as_ref())?.format("%+")).into());
		map.insert("name".into(),folder.name.as_str().into());
		map.insert("parentId".into(),folder.parent_id.as_ref().map(|c|serde_json::Value::String(c.to_string())).unwrap_or(serde_json::Value::Null));
		if detail{
			use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
			use diesel_async::RunQueryDsl;
			let folders_count:i64={
				use crate::models::drive_folder::drive_folder::dsl::drive_folder;
				use crate::models::drive_folder::drive_folder::dsl::*;
				drive_folder.filter(parentId.eq(folder.id.as_str())).count().get_result(con).await.map_err(|e|{
					eprintln!("{:?}",e);
				})
			}.ok()?;
			map.insert("foldersCount".into(),folders_count.into());
			let files_count:i64={
				use crate::models::drive_file::drive_file::dsl::drive_file;
				use crate::models::drive_file::drive_file::dsl::*;
				drive_file.filter(folderId.eq(folder.id.as_str())).count().get_result(con).await.map_err(|e|{
					eprintln!("{:?}",e);
				})
			}.ok()?;
			map.insert("filesCount".into(),files_count.into());
		}
		Some(map)
/* 
		return await awaitAll({

			...(opts.detail ? {

				...(folder.parentId ? {
					parent: this.pack(folder.parentId, {
						detail: true,
					}),
				} : {}),
			} : {}),
		});
	*/
	}

	fn get_thumbnail_url(&self,file: &MiDriveFile)->Option<String>{
		if file.mime_type.starts_with("video") {
			if let Some(thumbnail_url)=file.thumbnail_url.clone(){
				return Some(thumbnail_url);
			}
	
			//return this.videoProcessingService.getExternalVideoThumbnailUrl(file.webpublicUrl ?? file.url);
			return None;
		} else if file.uri.is_some() && file.user_host.is_some() && self.config.media_proxy.is_some() {
			// 動画ではなくリモートかつメディアプロキシ
			return Some(self.get_proxied_url(file.uri.as_ref().unwrap().as_str(), Some("static")));
		}
	
		if file.uri.is_some() && file.is_link && self.config.proxy_remote_files.unwrap_or(false) {
			// リモートかつ期限切れはローカルプロキシを試みる
			// 従来は/files/${thumbnailAccessKey}にアクセスしていたが、
			// /filesはメディアプロキシにリダイレクトするようにしたため直接メディアプロキシを指定する
			return Some(self.get_proxied_url(file.uri.as_ref().unwrap().as_str(), Some("static")));
		}
	
		let url = file.webpublic_url.as_ref().unwrap_or(&file.url);
		let convertible=[
			"image/jpeg",
			"image/tiff",
			"image/png",
			"image/gif",
			"image/apng",
			"image/vnd.mozilla.apng",
			"image/webp",
			"image/avif",
			"image/svg+xml",
		];
		if file.thumbnail_url.is_some(){
			return file.thumbnail_url.clone();
		}
		if convertible.contains(&file.mime_type.as_str()){
			Some(url.clone())
		}else{
			None
		}
	}
	fn get_proxied_url(&self,url: &str, mode: Option<&str>)-> String {
		if let Some(media_proxy)=self.config.media_proxy.as_ref(){
			let mut s=format!("{}/{}.webp?url={}",media_proxy,mode.unwrap_or("image"),url);
			if let Some(mode)=mode{
				s+="&";
				s+=mode;
				s+="=1";
			}
			s
		}else{
			url.to_owned()
		}
	}
	fn get_public_url(&self,file: &MiDriveFile, mode: Option<&str>, ap: bool)-> String { // static = thumbnail
		// PublicUrlにはexternalMediaProxyEnabledでもremoteProxyを使う
		// https://github.com/yojo-art/cherrypick/issues/84
		if file.uri.is_some() && file.user_host.is_some() && mode.is_none() && self.config.remote_proxy.is_some() {
			let key = file.webpublic_access_key.as_ref();
			if key.is_some() && key.unwrap().find('/').is_none() {	// 古いものはここにオブジェクトストレージキーが入ってるので除外
				if self.config.remote_proxy.as_ref().unwrap().starts_with("/") {
					return format!("{}{}/{}",self.config.url,self.config.remote_proxy.as_ref().unwrap(),key.unwrap());
				}
				return format!("{}/{}",self.config.remote_proxy.as_ref().unwrap(),key.unwrap());
			}
		}
		// リモートかつメディアプロキシ
		if file.uri.is_some() && file.user_host.is_some() && self.config.media_proxy.is_some() {
			return self.get_proxied_url(file.uri.as_ref().unwrap(), mode);
		}
	
		// リモートかつ期限切れはローカルプロキシを試みる
		if file.uri.is_some() && file.is_link && self.config.proxy_remote_files.unwrap_or(false){
			let key = file.webpublic_access_key.as_ref();
	
			if key.is_some() && key.unwrap().find('/').is_none() {	// 古いものはここにオブジェクトストレージキーが入ってるので除外
				let url = format!("{}/files/{}",self.config.url,key.unwrap());
				if mode == Some("avatar"){
					return self.get_proxied_url(file.uri.as_ref().unwrap(), Some("avatar"));
				}
				return url;
			}
		}

		let url = file.webpublic_url.as_ref().unwrap_or(&file.url);
	
		if mode == Some("avatar"){
			return self.get_proxied_url(file.uri.as_ref().unwrap(), Some("avatar"));
		}
	
		if ap && self.config.ap_file_base_url.is_some() {
			let ap_file_base_url = self.config.ap_file_base_url.as_ref().unwrap();
			match(reqwest::Url::from_str(ap_file_base_url.as_str()),reqwest::Url::from_str(url.as_str())){
				(Ok(ap_file_base_url),Ok(mut url))=>{
					let host=url.set_host(ap_file_base_url.host_str());
					let scheme=url.set_scheme(ap_file_base_url.scheme());
					url.set_path(&format!("{}{}",ap_file_base_url.path(),url.path()));
					if host.is_ok()&&scheme.is_ok(){
						return url.to_string();
					}
				},
				_=>{}
			}
		}

		return url.clone();
	}
}

fn get_public_properties(file: &MiDriveFile)-> FileProperties {
	let mut properties = file.properties.clone();
	if let Some(orientation)=file.properties.orientation {
		if orientation >= 5 {
			[properties.width, properties.height] = [properties.height, properties.width];
		}
		properties.orientation = None;
		return properties;
	}
	properties
}
fn is_media_silenced_host(silenced_hosts: &Vec<String>, host: Option<&str>)-> bool{
	if silenced_hosts.is_empty() || host.is_none(){
		return false;
	}
	let x=host.unwrap().to_lowercase();
	silenced_hosts.contains(&x)
}
pub async fn fetch_folder(con:&mut DBConnection<'_>,folder_id: Option<&str>,user_id: Option<&str>)-> Option<MiDriveFolder> {
	use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
	use diesel_async::RunQueryDsl;
	let folder_id=folder_id?;
	let res:MiDriveFolder={
		use crate::models::drive_folder::drive_folder::dsl::drive_folder;
		use crate::models::drive_folder::drive_folder::dsl::*;
		drive_folder.filter(userId.eq(user_id)).filter(id.eq(folder_id)).select(MiDriveFolder::as_select()).first(con).await.map_err(|e|{
			eprintln!("{:?}",e);
		})
	}.ok()?;
	Some(res)
}
pub async fn calc_drive_usage_of(con:&mut DBConnection<'_>,user_id: &str)-> i64 {
	use diesel::dsl::sum;
	use models::drive_file::drive_file::dsl::drive_file;
	use models::drive_file::drive_file::dsl::*;
	use bigdecimal::ToPrimitive;
	use diesel::{ExpressionMethods, QueryDsl};
	use diesel_async::RunQueryDsl;
	//64bit拡張の値はすべて読む
	let size_long_sum:Option<i64>=drive_file.filter(userId.eq(user_id)).filter(isLink.eq(false)).select(sum(size_long)).first::<Option<bigdecimal::BigDecimal>>(con).await.map_err(|e|{
		eprintln!("{:?}",e);
	}).ok().unwrap_or_default().map(|a|a.to_i64()).unwrap_or_default();
	//32bit基本の値は64bit値が0の物のみ
	let size_sum:Option<i64>=drive_file.filter(userId.eq(user_id)).filter(isLink.eq(false)).filter(size_long.eq(0)).select(sum(size)).first::<Option<i64>>(con).await.map_err(|e|{
		eprintln!("{:?}",e);
	}).ok().unwrap_or_default();
	size_long_sum.unwrap_or(0)+size_sum.unwrap_or(0)
}
fn validate_file_name(name: &str)-> bool {
	return 
		(name.trim().len() > 0) &&
		(name.len() <= 200) &&
		(name.find('\\') == None) &&
		(name.find('/') == None) &&
		(name.find("..") == None)
	;
}
/**
 * 与えられた拡張子とファイル名が一致しているかどうかを確認し、
 * 一致していない場合は拡張子を付与して返す
 *
 * extはfile-typeのextを想定
 */
fn correct_filename(filename: &str, ext: Option<&str>) ->String{
	let dot_ext = if let Some(ext)=ext{
		if ext.starts_with("."){
			ext.to_owned()
		}else{
			format!(".{}",ext)
		}
	}else{
		".unknown".to_owned()
	};

	let mut split = filename.split('.');
	let filename_ext=split.next().map(|s|s.to_lowercase());
	if filename_ext.is_none() {
		// filenameが拡張子を持っていない場合は拡張子をつける
		return format!("{}{}",filename,dot_ext);
	}
	let filename_ext=filename_ext.unwrap();
	const TARGET_EXTS_TO_SKIP: [&str; 7] = [
		".7z",
		".bz2",
		".gz",
		".tar",
		".tgz",
		".xz",
		".zip",
	];
	
	if ext.is_none() ||// 未知のファイル形式かつ拡張子がある場合は何もしない
		// 拡張子が一致している場合は何もしない
		filename_ext == dot_ext ||

		// jpeg, tiffを同一視
		dot_ext == ".jpg" && filename_ext == ".jpeg" ||
		dot_ext == ".tif" && filename_ext == ".tiff" ||
		// dllもexeもportable executableなので判定が正しく行われない
		dot_ext == ".exe" && filename_ext == ".dll" ||

		// 圧縮形式っぽければ下手に拡張子を変えない
		// https://github.com/misskey-dev/misskey/issues/11482
		TARGET_EXTS_TO_SKIP.binary_search(&dot_ext.as_str()).is_ok()
	{
		return filename.to_owned();
	}

	// 拡張子があるが一致していないなどの場合は拡張子を付け足す
	format!("{}{}",filename,dot_ext)
}
