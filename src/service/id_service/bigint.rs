use std::num::ParseIntError;

use num::FromPrimitive;

fn parse_big_int_chunked(
	mut s: &str,
	chars: &'static str,
	chunk_size: usize,
	power_of_chunk_size: num::BigInt,
) -> Result<num::BigInt, &'static str> {
	let mut chunks = vec![];
	while !s.is_empty() {
		let (a, b) = s.split_at(0.max(s.len() as isize - chunk_size as isize) as usize);
		chunks.push(b);
		s = a;
	}
	let mut result = num::BigInt::from_i32(0).unwrap();
	for chunk in chunks.iter().rev() {
		result *= power_of_chunk_size.clone();

		let mut n = 0;
		for c in chunk.chars().into_iter() {
			n = n * 32 + chars.find(c).ok_or("parse_big_int_chunked")? as i64;
		}
		//FIXME: 挙動が怪しい
		/*
		let n = i128::from_str_radix(chunk, base).map_err(|e|{
			eprintln!("{}:{} {:?} {}@{}",file!(),line!(),e,chunk,base);
			e
		})?;
		*/
		result += n;
	}
	println!("{}", result.to_string());
	Ok(result)
}

pub fn parse_big_int36(s: &str) -> Result<num::BigInt, &'static str> {
	// log_36(Number.MAX_SAFE_INTEGER) => 10.251599391715352
	// so we process 10 chars at once
	parse_big_int_chunked(
		&s.to_ascii_uppercase(),
		"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ",
		10,
		num::BigInt::from_i32(36).unwrap().pow(10),
	)
}

pub fn parse_big_int16(s: &str) -> Result<num::BigInt, &'static str> {
	// log_16(Number.MAX_SAFE_INTEGER) => 13.25
	// so we process 13 chars at once
	parse_big_int_chunked(
		&s.to_ascii_uppercase(),
		"0123456789ABCDEF",
		13,
		num::BigInt::from_i32(16).unwrap().pow(13),
	)
}

pub fn parse_big_int32(s: &str) -> Result<num::BigInt, &'static str> {
	// log_32(Number.MAX_SAFE_INTEGER) => 10.6
	// so we process 10 chars at once
	parse_big_int_chunked(
		s,
		"0123456789ABCDEFGHJKMNPQRSTVWXYZ",
		10,
		num::BigInt::from_i32(32).unwrap().pow(10),
	)
}
