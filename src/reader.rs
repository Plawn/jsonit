use {anyhow::Result as InternalResult, serde::de::DeserializeOwned, std::io::Read};


pub struct JsonSeqIterator<'a, R, O> {
	state: State<'a>,
	reader: R,
	output_type: std::marker::PhantomData<O>,
}
enum State<'a> {
	NotStarted { path_to_look_for: &'a str },
	Started,
	Ended,
}

impl<'a, R: Read, O: DeserializeOwned> JsonSeqIterator<'a, R, O> {
	pub fn new(reader: R, path_to_look_for: &'a str) -> Self {
		Self {
			state: State::NotStarted { path_to_look_for },
			reader,
			output_type: std::marker::PhantomData,
		}
	}

	pub fn next_char(&mut self) -> InternalResult<u8> {
		let mut buf = [0_u8; 1];
		self.reader.read_exact(&mut buf)?;
		Ok(buf[0])
	}

	fn deserialize_one_item(&mut self, v: Option<u8>) -> InternalResult<O> {
		if let Some(w) = v {
			let r = &[w][..];
			O::deserialize(&mut serde_json::Deserializer::from_reader(
				&mut r.chain(self.reader.by_ref()),
			))
			.map_err(|e| e.into())
		} else {
			O::deserialize(&mut serde_json::Deserializer::from_reader(&mut self.reader)).map_err(|e| e.into())
		}
	}
}

impl<'a, R: Read, O: DeserializeOwned> Iterator for JsonSeqIterator<'_, R, O> {
	type Item = InternalResult<O>;
	fn next(&mut self) -> Option<Self::Item> {
		match self.state {
			State::NotStarted { path_to_look_for: _ } => {
				// TODO advance the reader to the path. As a stub:
				loop {
					match self.next_char() {
						Err(e) => return Some(Err(e)),
						Ok(b'[') => {
							self.state = State::Started;
							return Some(self.deserialize_one_item(None));
						}
						Ok(_) => {
							// Wait until we find our inner array
						}
					}
				}
			}
			State::Started => loop {
				break match self.next_char() {
					Err(e) => Some(Err(e)),
					Ok(c) => match c {
						b']' => {
							self.state = State::Ended;
							None
						}
						b',' => {
							// Parse with serde_json
							Some(self.deserialize_one_item(None))
						}
						w => {
							if w.is_ascii_whitespace() {
								continue;
							} else if w.is_ascii_digit() || w == b'n' { // n for null
								// handle serde eating one too many char
								// deserialyze number
								Some(self.deserialize_one_item(Some(w)))
							} else if w == b'}' || w == b']' {
								// suppose end
								None
							} else {
								Some(Err(anyhow::anyhow!("[JsonIt] Unexpected character: {}", char::from(w))))
							}
						}
					},
				};
			},
			State::Ended => None,
		}
	}
}
