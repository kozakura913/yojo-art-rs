use std::sync::{atomic::AtomicI64, Arc};

use axum::{extract::ws::WebSocketUpgrade, http::StatusCode, response::IntoResponse};
use futures::{stream::{SplitSink, SplitStream}, SinkExt, StreamExt, TryStreamExt};
use reqwest::{header::USER_AGENT, Client};
use serde::Deserialize;
use tokio::io::AsyncReadExt;
use tokio_util::io::StreamReader;

use crate::Context;

pub async fn post(
	ctx:Context,
	request: axum::extract::Request,
)->axum::response::Response{
	let path=request.uri().path();
	let mut url=format!("{}{}",ctx.config.backend,path);
	if let Some(query)=request.uri().query(){
		url+="?";
		url+=query;
	}
	let headers=request.headers();
	println!("[{}] \"POST {}\" \"{:?}\"",chrono::Utc::now().format("%+"),url,headers.get(USER_AGENT).map(|s|s.to_str().map(|s|s.replace("\"","'"))));
	let builder=ctx.client.post(url);
	let builder=builder.headers(headers.clone());
	let stream=request.into_body().into_data_stream();
	let body_with_io_error = stream.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err));
	let mut body_reader = StreamReader::new(body_with_io_error);
	let mut buf=vec![];
	if let Err(e)=body_reader.read_to_end(&mut buf).await{
		eprintln!("{}:{} {:?}",file!(),line!(),e);
		return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
	}
	let builder=builder.body(buf);
	let res=builder.send().await;
	match res{
		Ok(v)=>{
			let mut header=axum::http::header::HeaderMap::new();
			for (k,v) in v.headers(){
				header.append(k,v.clone());
			}
			let status=v.status().as_u16();
			let body=v.bytes().await.map(|b|b.to_vec()).unwrap_or_default();
			(StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY),header,body).into_response()
		},
		Err(e)=>{
			eprintln!("{:?}",e);
			StatusCode::BAD_GATEWAY.into_response()
		}
	}
}
#[derive(Debug, Deserialize)]
pub struct StreamingParams{
	#[serde(rename = "i")]
	token:Option<String>,
}
pub async fn streaming(
	ctx:Context,
	ws: WebSocketUpgrade,
	axum::extract::Query(q):axum::extract::Query<StreamingParams>,
)->axum::response::Response{
	ws.on_upgrade(|socket| handle_socket(socket, ctx,q))
}
async fn handle_socket(
	socket: axum::extract::ws::WebSocket,
	ctx:Context,
	q:StreamingParams,
) {
	let (sender, receiver) = socket.split();
	let backend=ws_backend(ctx.client.clone(),&ctx.config.backend,q.token.as_deref()).await;
	match backend{
		Ok(backend)=>{
			let (backend_sender, backend_receiver) = backend.split();
			let ping_sync=Arc::new(AtomicI64::new(chrono::Utc::now().timestamp_millis()));
			let sender_handle = tokio::spawn(ws_write_side(sender, backend_receiver,ctx.clone(),ping_sync.clone()));
			let reciever_handle = tokio::spawn(ws_read_side(receiver, sender_handle.abort_handle(),backend_sender, ctx.clone(),ping_sync));
			let _=reciever_handle.await;
		},
		Err(e)=>{
			eprintln!("backend ws error {:?}",e);
		}
	}
}

async fn ws_read_side(
	mut receiver: SplitStream<axum::extract::ws::WebSocket>,
	sender_handle: tokio::task::AbortHandle,
	mut backend_sender: SplitSink<reqwest_websocket::WebSocket, reqwest_websocket::Message>,
	ctx:Context,
	ping_sync:Arc<AtomicI64>,
) {
	//80秒pingが無ければ、接続が切れたと判定して処理を終了する
	let timeout = std::time::Duration::from_secs(80);

	loop {
		let result = tokio::time::timeout(timeout, receiver.next()).await;
		if sender_handle.is_finished(){
			return;
		}
		match result {
			// A new message has been received
			Ok(Some(Ok(message))) => match message {
				axum::extract::ws::Message::Close(_frame) => {
					println!("close from client");
					break;
				}
				_ => {
					let message=message.into_text();
					//println!("WS from client {:?}",message);
					if let Ok(message)=message{
						let res = backend_sender.send(reqwest_websocket::Message::Text(message)).await;
						if let Err(e)=res{
							eprintln!("WS send to backend error {:?}",e);
							break;
						}else{
							ping_sync.store(chrono::Utc::now().timestamp_millis(), std::sync::atomic::Ordering::Relaxed);
						}
					}
				}
			},
			// An error occurred while trying to read a message
			Ok(Some(Err(e))) => {
				eprintln!("ReadMessageError: {:?}", e);
				break;
			}
			Ok(None) => {
				println!("receive nothing.");
			}
			// Timeout occurred
			Err(e ) => {
				println!("from client timeout! {:?}", e);
				break;
			}
		}
	}
	ping_sync.store(0, std::sync::atomic::Ordering::Relaxed);
	//receive処理を抜けるタイミングで、sender側の処理も終了する
	sender_handle.abort();
}
async fn ws_write_side(
	mut sender: SplitSink<axum::extract::ws::WebSocket,axum::extract::ws::Message>,
	mut backend_receiver: SplitStream<reqwest_websocket::WebSocket>,
	ctx:Context,
	ping_sync:Arc<AtomicI64>,
) {
	let timeout = std::time::Duration::from_secs(100);
	loop {
		let result = tokio::time::timeout(timeout, backend_receiver.next()).await;
		match result {
			// A new message has been received
			Ok(Some(Ok(message))) => match message {
				reqwest_websocket::Message::Text(message) => {
					//println!("WS from backend {:?}",message);
					let res = sender.send(axum::extract::ws::Message::Text(message)).await;
					if let Err(e)=res{
						eprintln!("WS send to client error {:?}",e);
						break;
					}
				},
				_=>{
					//
				}
			},
			// An error occurred while trying to read a message
			Ok(Some(Err(e))) => {
				eprintln!("ReadMessageError: {:?}", e);
				break;
			}
			Ok(None) => {
				println!("receive nothing.");
			}
			// Timeout occurred
			Err(e ) => {
				let last_send=ping_sync.load(std::sync::atomic::Ordering::Relaxed);
				let now=chrono::Utc::now().timestamp_millis();
				if now-60*1000 < last_send{
					continue;
				}
				println!("from backend timeout! {:?}", e);
				break;
			}
		}
	}
}
pub async fn get(
	ctx:Context,
	request: axum::extract::Request,
)->axum::response::Response{
	let path=request.uri().path();
	let mut url=format!("{}{}",ctx.config.backend,path);
	if let Some(query)=request.uri().query(){
		url+="?";
		url+=query;
	}
	let headers=request.headers();
	println!("[{}] \"GET {}\" \"{:?}\"",chrono::Utc::now().format("%+"),url,headers.get(USER_AGENT).map(|s|s.to_str().map(|s|s.replace("\"","'"))));
	let builder=ctx.client.get(url);
	let builder=builder.headers(headers.clone());
	let res=builder.send().await;
	match res{
		Ok(v)=>{
			let mut header=axum::http::header::HeaderMap::new();
			for (k,v) in v.headers(){
				match k.as_str(){
					"transfer-encoding"|"connection"=>{},
					_=>{
						header.append(k,v.clone());
						//println!("header {} {:?}",k,v);
					}
				}
			}
			let status=v.status().as_u16();
			let body=v.bytes().await.map(|b|b.to_vec()).unwrap_or_default();
			(StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY),header,body).into_response()
		},
		Err(e)=>{
			eprintln!("{:?}",e);
			StatusCode::BAD_GATEWAY.into_response()
		}
	}
}

async fn ws_backend(client:Client,backend_url:&str,token:Option<&str>)->Result<reqwest_websocket::WebSocket,String>{
	use reqwest_websocket::RequestBuilderExt;
	let url=reqwest::Url::parse(backend_url);
	let mut url=match url {
		Ok(url)=>url,
		Err(e)=>{
			eprintln!("{:?}",e);
			panic!("URL parse error");
		}
	};
	if url.scheme()=="http"{
		url.set_scheme("ws").unwrap();
	}else{
		url.set_scheme("wss").unwrap();
	}
	url.set_path("streaming");
	if let Some(token)=token{
		let query=format!("i={}",token);
		url.set_query(Some(&query));
	}
	let response = client
		.get(url)
		.upgrade()
		.send()
		.await.map_err(|e|e.to_string())?;
	let websocket = response.into_websocket().await.map_err(|e|e.to_string())?;
	Ok(websocket)
}
