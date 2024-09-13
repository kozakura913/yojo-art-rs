// AIDX
// 長さ8の[2000年1月1日からの経過ミリ秒をbase36でエンコードしたもの] + 長さ4の[個体ID] + 長さ4の[カウンタ]
// (c) mei23
// https://misskey.m544.net/notes/71899acdcc9859ec5708ac24

use std::sync::atomic::AtomicI32;

use pad::{Alignment, PadStr};

use super::IdServiceImpl;

const TIME2000:i64 = 946684800000;
const TIME_LENGTH:usize = 8;
const NODE_LENGTH:usize = 4;
const NOISE_LENGTH:usize = 4;

#[derive(Debug)]
pub struct AidxService{
	node_id:String,
	counter:AtomicI32,
}
impl IdServiceImpl for AidxService{
	fn is_safe_t(&self,t:i64)->bool {
		t > TIME2000
	}
	fn gen(&self,time: i64)->String {
		get_time(time) + self.node_id.as_str() + &self.get_noise()
	}
	fn parse(&self,id: &str)->Option<i64>{
		Some(i64::from_str_radix(&id[0.. TIME_LENGTH], 36).ok()? + TIME2000)
	}
}
impl AidxService{
	pub fn new()->Self{
		let id=nanoid::nanoid!(NODE_LENGTH,&"0123456789abcdefghijklmnopqrstuvwxyz".chars().collect::<Vec<char>>());
		Self{
			node_id: id,
			counter: AtomicI32::new(0)
		}
	}
	fn get_noise(&self)->String {
		let counter=self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
		use num::FromPrimitive;
		let mut s=num::BigInt::from_i32(counter).unwrap().to_str_radix(36).pad(NOISE_LENGTH,'0', Alignment::Right,false);
		//非ASCII文字を含む場合は文字情報を取得する必要がある
		//let split_pos = s.char_indices().nth_back(1).unwrap().0;
		//s[split_pos..].to_string()
		//ASCII文字のみで構成される場合は簡単な計算で行える
		let len = s.chars().count() - NOISE_LENGTH;
		let _=s.drain(0..len);
		s
	}
}
fn get_time(time: i64)->String {
	let time = (time - TIME2000).max(0);
	use num::FromPrimitive;
	let mut s=num::BigInt::from_i64(time).unwrap().to_str_radix(36).pad(TIME_LENGTH,'0', Alignment::Right,false);
	//ASCII文字のみで構成される場合は簡単な計算で行える
	let len = s.chars().count() - TIME_LENGTH;
	let _=s.drain(0..len);
	s
}
