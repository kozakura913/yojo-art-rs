use std::time::Duration;

fn main() {
	println!("Hello, world!");
}
const BASE_URL:&'static str="http://localhost:3001/api";
#[test]
fn tl(){
	let resp=api_post("/signup",serde_json::json!({
		"username": uuid::Uuid::new_v4().to_string(),
		"password": "a",
	}));
	let token=resp.get("token").unwrap().as_str().unwrap();
	let resp=api_post("/notes/timeline",serde_json::json!({
		"withRenotes": true,
		"withCats": false,
		"limit": 10,
		"allowPartial": true,
		"i": token,
	}));
	assert!(resp.is_array(),"{:?}", resp);
}
fn api_post(endpoint:&str,req_body:serde_json::Value)->serde_json::Value{
	api_post_with_opt(endpoint, req_body, Default::default())
}
fn api_post_with_opt(endpoint:&str,req_body:serde_json::Value,opt:ApiOptions)->serde_json::Value{
	let c=reqwest::blocking::Client::new();
	let json=serde_json::to_string_pretty(&req_body).unwrap();
	let req=c.post(BASE_URL.to_owned()+endpoint);
	let req=req.header(reqwest::header::CONTENT_TYPE,"application/json");
	let req=req.timeout(opt.timeout);
	let req=req.body(json).build().unwrap();
	let res=c.execute(req).unwrap();
	if res.status().is_success(){
		serde_json::from_str(&res.text().unwrap()).unwrap()
	}else{
		panic!("status {}\nresp: {:?}",res.status(),res.text())
	}
}
#[derive(Clone,Debug)]
pub struct ApiOptions{
	timeout:Duration,
}
impl Default for ApiOptions{
	fn default() -> Self {
		Self {
			timeout: Duration::from_secs(1)
		}
	}
}
