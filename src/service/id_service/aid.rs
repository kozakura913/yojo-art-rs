// AID
// 長さ8の[2000年1月1日からの経過ミリ秒をbase36でエンコードしたもの] + 長さ2の[ノイズ文字列]
use std::sync::atomic::AtomicI16;

use rand::{Rng, SeedableRng};

use super::IdServiceImpl;

const TIME2000:i64 = 946684800000;
#[derive(Debug)]
pub struct AidService{
	counter:AtomicI16,
}
impl IdServiceImpl for AidService{
	fn is_safe_t(&self,t:i64)->bool {
		t > TIME2000
	}
	fn gen(&self,time: i64)->String {
		get_time(time) + &self.get_noise()
	}
	fn parse(&self,id: &str)->Option<i64>{
		Some(i64::from_str_radix(&id[0..8], 36).ok()? + TIME2000)
	}
}
impl AidService{
	pub fn new()->Self{
		let counter = rand::rngs::StdRng::from_entropy().gen::<i16>();
		Self{
			counter:AtomicI16::new(counter),
		}
	}
	
	fn get_noise(&self)->String {
		let counter=self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
		use num::FromPrimitive;
		let mut s=format!("{:0>2}",num::BigInt::from_i16(counter).unwrap().to_str_radix(36));
		//非ASCII文字を含む場合は文字情報を取得する必要がある
		//let split_pos = s.char_indices().nth_back(1).unwrap().0;
		//s[split_pos..].to_string()
		//ASCII文字のみで構成される場合は簡単な計算で行える
		let len = s.chars().count() - 2;
		let _=s.drain(0..len);
		s
	}
}
fn get_time(time: i64)->String {
	let time = (time - TIME2000).max(0);
	use num::FromPrimitive;
	format!("{:0>8}",num::BigInt::from_i64(time).unwrap().to_str_radix(36))
}
