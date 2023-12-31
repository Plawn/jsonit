use crate::utils::compare_stack_reader;

use {anyhow::Result as InternalResult, serde::de::DeserializeOwned, std::io::Read};

pub struct JsonSeqIterator<'a, R, O> {
	state: State<'a>,
	reader: R,
	output_type: std::marker::PhantomData<O>,
}
enum State<'a> {
	NotStarted { path_to_look_for: PrefixPath<'a> },
	Started,
	Ended,
}

pub type PrefixPath<'a> =  &'a [u8];

impl<'a, R: Read, O: DeserializeOwned> JsonSeqIterator<'a, R, O> {
	pub fn new(reader: R, path_to_look_for: &'a [u8]) -> Self {
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

#[derive(PartialEq, Debug, Copy, Clone)]
enum ParseValueType {
	String,
	Number,
	Null,
	Map,
	Array,
}

#[derive(PartialEq, Debug)]
enum NotStartedState {
	ParseObjectKey,
	ParseObject,
	ParseValue(ParseValueType),
	ExpectValue,
	/// We expect ":" or whitespace
	ExpectPoints,
	None,
}

impl<'a, R: Read, O: DeserializeOwned> Iterator for JsonSeqIterator<'_, R, O> {
	type Item = InternalResult<O>;
	fn next(&mut self) -> Option<Self::Item> {
		match self.state {
			State::NotStarted { path_to_look_for } => {
				// TODO advance the reader to the path. As a stub:
				let mut key_stack: Vec<Vec<u8>> = vec![];
				// the current key where we parse the value
				let mut current_key: Vec<u8> = vec![];

				// keeps state of the parsing
				let mut state = NotStartedState::None;
				// prevents rebuilding the key stack without rebuilding it
				let mut stack_dirty = false;
				// Keeps state if the next character is escaped
				let mut escape = false;
				loop {
					match self.next_char() {
						Err(e) => return Some(Err(e)),
						// Ok(b'[') => {}
						Ok(c) => {
							if stack_dirty && compare_stack_reader(&key_stack, path_to_look_for) {
								self.state = State::Started;
								// advance until we get the array
								let r = loop {
									match self.next_char() {
										// should have err
										Err(e) => {
											break Err(e);
										}
										Ok(c) => match c {
											b'[' => break Ok(()),
											_ => {
												continue;
											}
										},
									};
								};
								if r.is_ok() {
									return Some(self.deserialize_one_item(None));
								}
							}

							match state {
								// handle current key count
								NotStartedState::ParseObjectKey => {
									if c == b'\"' && !escape {
										state = NotStartedState::ExpectPoints;
										// TODO: should avoid cloning
										key_stack.push(current_key.clone());
										current_key.clear();
										stack_dirty = true;
									} else {
										current_key.push(c);
									}
								}
								NotStartedState::ParseValue(t) => {
									// detect end of value
									match t {
										ParseValueType::String => {
											if c == b'\"' && !escape {
												state = NotStartedState::ParseObject;
												key_stack.pop();
												stack_dirty = true;
											}
											if escape {
												escape = false;
											}
										}
										ParseValueType::Array => {
											if c == b']' {
												state = NotStartedState::ParseObject;
												key_stack.pop();
												stack_dirty = true;
											}
										}
										ParseValueType::Number => {
											if c == b',' {
												state = NotStartedState::ParseObject;
												key_stack.pop();
												stack_dirty = true;
											}
										}
										ParseValueType::Null => {
											if c == b',' {
												state = NotStartedState::ParseObject;
												key_stack.pop();
												stack_dirty = true;
											}
										}
										ParseValueType::Map => {
											// key_stack.push(current_key.clone());
											// current_key.clear();
											state = NotStartedState::ParseObject;
										}
									};
								}
								NotStartedState::ExpectValue => {
									if c == b'\"' {
										state = NotStartedState::ParseValue(ParseValueType::String);
									} else if c == b'n' {
										// speculative nominal value is null
										state = NotStartedState::ParseValue(ParseValueType::Null);
									} else if c == b'{' {
										state = NotStartedState::ParseValue(ParseValueType::Map);
									} else if c == b'[' {
										state = NotStartedState::ParseValue(ParseValueType::Array);
									} else if c == b' ' {
									} else {
										state = NotStartedState::ParseValue(ParseValueType::Number);
									}
								}
								NotStartedState::ExpectPoints => {
									if c == b':' {
										state = NotStartedState::ExpectValue;
									}
								}
								NotStartedState::ParseObject => {
									if c == b'\"' {
										state = NotStartedState::ParseObjectKey;
									}
									if c == b'}' {
										key_stack.pop();
										stack_dirty = true;
									}
								}
								NotStartedState::None => {
									if c == b'{' {
										// start root of object
										state = NotStartedState::ParseObject;
									} else if c == b'[' {
										todo!("arrays are unsupported for now");
									} else if c != b' ' {
										panic!("malformed");
									}
								}
							};
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
							} else if w.is_ascii_digit() || w == b'n' {
								// n for null
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
