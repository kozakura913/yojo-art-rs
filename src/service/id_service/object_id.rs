use pad::{Alignment, PadStr};

use super::IdServiceImpl;

const CHARS:&'static str = "0123456789abcdef";

#[derive(Debug)]
pub struct ObjectIdService;
impl IdServiceImpl for ObjectIdService{
	fn is_safe_t(&self,t:i64)->bool {
		t > 0
	}
	fn gen(&self,time: i64)->String {
		let random=nanoid::nanoid!(16,&CHARS.chars().collect::<Vec<char>>());
		get_time(time) + &random
	}
	fn parse(&self,id: &str)->Option<i64> {
		Some(i64::from_str_radix(&id[0..8], 16).ok()? * 1000)
	}
}
impl ObjectIdService{
	pub fn new()->Self{
		Self
	}
}

fn get_time(time: i64) ->String{
	let time = time.max(0);
	if time == 0 {
		return CHARS[0..1].to_string();
	}

	let time=(time as f64 / 1000.0).floor() as i64;

	use num::FromPrimitive;
	num::BigInt::from_i64(time).unwrap().to_str_radix(16).pad(8,'0',Alignment::Right,false)
}
