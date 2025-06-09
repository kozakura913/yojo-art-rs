use std::time::Duration;

fn main() {
	println!("Hello, world!");
}
const BASE_URL:&'static str="http://localhost:3001/api";
#[test]
fn note_create(){
	let (_,token)=signup();
	let resp=api_post("/notes/create",serde_json::json!({
		"text": "a",
		"poll": null,
		"tagText": null,
		"event": null,
		"cw": null,
		"visibility": "public",
		"reactionAcceptance": null,
		"disableRightClick": false,
		"scheduledDelete": null,
		"i": token,
	}));
	assert!(resp.get("createdNote").is_some(),"{:?}", resp);
}
#[test]
fn timeline(){
	let (_,token)=signup();
	let resp=api_post("/notes/create",serde_json::json!({
		"text": "a",
		"poll": null,
		"tagText": null,
		"event": null,
		"cw": null,
		"visibility": "public",
		"reactionAcceptance": null,
		"disableRightClick": false,
		"scheduledDelete": null,
		"i": token,
	}));
	let resp=api_post("/notes/timeline",serde_json::json!({
		"withRenotes": true,
		"withCats": false,
		"limit": 10,
		"allowPartial": true,
		"i": token,
	}));
	assert!(resp.is_array(),"{:?}", resp);
	assert_eq!(resp.as_array().unwrap().len(),1,"{:?}", resp);
}
fn signup()->(String,String){
	let mut rng = rand::rng();
	use rand::distr::SampleString;
	let username = rand::distr::Alphanumeric.sample_string(&mut rng, 16);
	let password = rand::distr::Alphanumeric.sample_string(&mut rng, 16);
	let resp=api_post("/signup",serde_json::json!({
		"username": &username,
		"password": password,
	}));
	let token=resp.get("token").unwrap().as_str().unwrap().to_owned();
	(username,token)
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
