use axum::BoxError;
use base64::Engine;
use std::{collections::HashMap, default, str::FromStr, sync::Arc};

use crate::{DataBase, ParsedMisskeyConfig, models::user_keypair::MiUserKeypair};

#[derive(Clone, Debug)]
pub struct APSignatureService {
	db: DataBase,
	client: reqwest::Client,
	misskey_config: Arc<ParsedMisskeyConfig>,
}
pub fn sha256_digest(body: &[u8]) -> String {
	use sha2::Digest;
	use sha2::Sha256;
	let mut hasher = Sha256::new();
	hasher.update(&body);
	format!(
		"SHA-256={}",
		base64::engine::general_purpose::STANDARD.encode(&hasher.finalize())
	)
}
impl APSignatureService {
	pub fn new(
		db: DataBase,
		client: reqwest::Client,
		misskey_config: Arc<ParsedMisskeyConfig>,
	) -> Self {
		Self {
			db,
			client,
			misskey_config,
		}
	}
	//url="https://federation-test-temp-bettaku_engawa.penginn.net/users/01JWB93KKPB4KMB4VFEACTAJFM"
	pub async fn get(&self, url: impl AsRef<str>, me_id: impl AsRef<str>) {
		let date = chrono::Utc::now()
			.format("%a, %d %b %Y %H:%M:%S %Z")
			.to_string();
		let res = self.signature(url, &date, me_id.as_ref(), None, None).await;
		if let Ok(res) = res {
			println!("{:?}", res.text().await);
		}
	}
	pub async fn post(
		&self,
		inbox: impl AsRef<str>,
		me_id: impl AsRef<str>,
		sha256_digest: &String,
		body: impl Into<tokio_util::bytes::Bytes>,
	) -> Result<reqwest::Response, BoxError> {
		let date = chrono::Utc::now()
			.format("%a, %d %b %Y %H:%M:%S %Z")
			.to_string();
		let body: tokio_util::bytes::Bytes = body.into();
		self.signature(
			inbox,
			&date,
			me_id.as_ref(),
			Some(&sha256_digest),
			Some(body.clone()),
		)
		.await
	}
	/*
	pub async fn post_all<T>(&self,inbox:impl Iterator<Item=T>,me_id:impl AsRef<str>,body:impl Into<tokio_util::bytes::Bytes>)->Vec<Result<reqwest::Response,ServerError>> where T:AsRef<str>{
		let date=chrono::Utc::now().format("%a, %d %b %Y %H:%M:%S %Z").to_string();
		let body:tokio_util::bytes::Bytes=body.into();
		let digest=sha256_digest(&body);
		let mut job=vec![];
		for s in inbox{
			job.push(self.signature(s,&date,me_id.as_ref(),Some(&digest),Some(body.clone())));
		}
		futures_util::future::join_all(job).await
	}
	*/
	async fn signature(
		&self,
		url: impl AsRef<str>,
		date: &str,
		user_id: &str,
		digest: Option<&String>,
		body: Option<tokio_util::bytes::Bytes>,
	) -> Result<reqwest::Response, BoxError> {
		let url = reqwest::Url::from_str(url.as_ref())?;
		let host = url.host_str().ok_or("unknown host")?.to_owned();
		let method = if body.is_some() { "post" } else { "get" };
		let request_target = format!("{method} {}{}", url.path(), url.query().unwrap_or(""));
		let mut sign_headers = format!("(request-target) host date");
		let mut sign_target =
			format!("(request-target): {request_target}\nhost: {host}\ndate: {date}\n");
		let accept = "application/activity+json, application/ld+json; profile=\"https://www.w3.org/ns/activitystreams\"";
		let request = if let Some(body) = body {
			let request = self.client.post(url);
			let request = if let Some(digest) = &digest {
				sign_target += "digest: ";
				sign_target += digest;
				sign_target += "\n";
				sign_target += "content-type: application/activity+json";
				sign_headers += " digest";
				sign_headers += " content-type";
				let request = request.header("digest", digest.as_str());
				request.header("content-type", "application/activity+json")
			} else {
				request
			};
			request.body(body)
		} else {
			let request = self.client.get(url);
			sign_target += "accept: ";
			sign_target += accept;
			sign_headers += " accept";
			let request = request.header("accept", accept);
			request
		};
		let mut dbcon = self.db.get_read_only().await?;
		let keypair = MiUserKeypair::load_by_user(&mut dbcon, user_id).await?;
		let private_key = rsa::RsaPrivateKey::from_pkcs8_pem(&keypair.private_key)?;
		use rsa::signature::SignatureEncoding;
		use rsa::{pkcs8::DecodePrivateKey, signature::SignerMut};
		let mut signing_key = rsa::pkcs1v15::SigningKey::<rsa::sha2::Sha256>::new(private_key);
		let signature = signing_key.sign(sign_target.as_bytes());
		let signature = base64::engine::general_purpose::STANDARD.encode(&signature.to_bytes());
		let key_id = format!("{}users/{}#main-key", self.misskey_config.url, user_id);
		let signature = format!(
			"keyId=\"{key_id}\",algorithm=\"rsa-sha256\",headers=\"{sign_headers}\",signature=\"{signature}\""
		);
		let mut request = request.build()?;
		let headers = request.headers_mut();
		headers.append("host", host.parse()?);
		headers.append("date", date.parse()?);
		headers.append("Authorization", format!("Signature {signature}").parse()?);
		headers.append("Signature", signature.parse()?);
		let res = self.client.execute(request).await?;
		Ok(res)
	}
}
