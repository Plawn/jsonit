use std::io::Read;
use anyhow::Result as InternalResult;


pub fn make_prefix(prefix: &str) -> Vec<u8> {
	let e = prefix.split('.');
	
	e
		.map(|e| e.as_bytes())
		.flat_map(|e| e.to_owned())
		.collect::<Vec<u8>>()
}

pub fn make_path(prefix: &str) -> Box<[u8]>  {
	prefix.split('.')
		.map(|e| e.as_bytes())
		.flat_map(|e| e.to_owned())
		.collect::<Vec<u8>>()
		.into_boxed_slice()
}


pub struct ReaderIter<R> {
	reader: R,
}

impl<R: Read> ReaderIter<R> {
	pub fn new(reader: R) -> Self {
		Self { reader }
	}

	pub fn next_char(&mut self) -> InternalResult<u8> {
		let mut buf = [0_u8; 1];
		self.reader.read_exact(&mut buf)?;
		Ok(buf[0])
	}
}

impl<R: Read> Iterator for ReaderIter<R> {
	type Item = InternalResult<u8>;
	fn next(&mut self) -> Option<Self::Item> {
		Some(self.next_char())
	}
}


pub fn compare_stack(stack: &[Vec<u8>], prefix: &Vec<u8>) -> bool {
	stack
		.iter()
		.flatten()
		.zip(prefix.iter())
		.take_while(|(a, b)| a == b)
		.count() == prefix.len()
}

pub fn compare_stack_reader(stack: &[Vec<u8>], prefix: &[u8]) -> bool {
	stack
		.iter()
		.flatten()
		.zip(prefix.iter())
		.take_while(|(a, b)| a == b)
		.count() == prefix.len()
}
