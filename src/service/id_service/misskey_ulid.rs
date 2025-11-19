use std::{sync::Mutex, time::SystemTime};

use rand::{Rng, SeedableRng, rngs::StdRng};

use super::IdServiceImpl;

// Crockford's Base32
// https://github.com/ulid/spec#encoding
const CHARS: &'static str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

#[derive(Debug)]
pub struct UlidService {
	rng: Mutex<StdRng>,
}
impl IdServiceImpl for UlidService {
	fn is_safe_t(&self, t: i64) -> bool {
		t > 0
	}
	fn gen_id(&self, time: i64) -> String {
		let mut rng = self.rng.lock().unwrap();
		let datetime = chrono::DateTime::from_timestamp_millis(time).unwrap();
		let rng: &mut StdRng = &mut rng;
		let timestamp = SystemTime::from(datetime)
			.duration_since(SystemTime::UNIX_EPOCH)
			.unwrap_or(std::time::Duration::ZERO)
			.as_millis();
		let timebits = (timestamp & ((1 << ulid::Ulid::TIME_BITS) - 1)) as u64;
		let msb = timebits << 16 | u64::from(rng.random::<u16>());
		let lsb = rng.random::<u64>();
		let id = ulid::Ulid::from((msb, lsb));
		id.to_string()
	}
	fn parse(&self, id: &str) -> Option<i64> {
		let timestamp = &id[0..10];
		let mut time = 0;
		for c in timestamp.chars().into_iter() {
			time = time * 32 + CHARS.find(c)? as i64;
		}
		Some(time)
	}
	fn parse_full(&self, id: &str) -> Option<(i64, num::BigInt)> {
		Some((
			self.parse(id)?,
			super::bigint::parse_big_int32(&id[10..26]).ok()?,
		))
	}
}
impl UlidService {
	pub fn new() -> Self {
		Self {
			rng: Mutex::new(StdRng::from_os_rng()),
		}
	}
}
