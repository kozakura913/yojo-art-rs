
//  4bit Fixed hex value 'g'
// 44bit UNIX Time ms in Hex
// 48bit Random value in Hex

use pad::{Alignment, PadStr};

use super::IdServiceImpl;

const CHARS:&'static str = "0123456789abcdef";

#[derive(Debug)]
pub struct MeidgService;
impl IdServiceImpl for MeidgService{
	fn is_safe_t(&self,t:i64)->bool {
		t > 0
	}
	fn gen(&self,time: i64)->String {
		let random=nanoid::nanoid!(12,&CHARS.chars().collect::<Vec<char>>());
		format!("g{}{}",get_time(time),&random)
	}
	fn parse(&self,id: &str)->Option<i64> {
		i64::from_str_radix(&id[1..12], 16).ok()
	}
}
impl MeidgService{
	pub fn new()->Self{
		Self
	}
}
fn get_time(time: i64)->String{
	let time=time.max(0);
	if time == 0 {
		return CHARS[0..1].to_string();
	}
	use num::FromPrimitive;
	num::BigInt::from_i64(time).unwrap().to_str_radix(16).pad(11,'0',Alignment::Right,false)
}
