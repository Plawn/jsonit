use serde::de::DeserializeOwned;


use crate::utils::{make_prefix, compare_stack};

fn fold_and_parse<T>(iterator: impl Iterator<Item = Delimiter>) -> impl Iterator<Item = serde_json::Result<T>>
where
	T: DeserializeOwned,
{
	let mut v: Vec<u8> = vec![];

	iterator.filter_map(move |e| match e {
		Delimiter::Item(e) => {
			v.push(e);
			None
		}
		Delimiter::End(e) => {
			v.push(e.get_end());
			let r = Some(serde_json::from_slice::<T>(&v));
			v.clear();
			r
		}
		// should never arrive here
		Delimiter::Stop => panic!("Hum, we should never be here, got stop"),
		// should never arrive here
		Delimiter::Skip => None,
		Delimiter::Start(e) => {
			v.push(e.get_start());
			None
		}
	})
}

#[derive(PartialEq, Debug)]
enum StructType {
	Map,
	Array,
}

impl StructType {
	fn get_start(&self) -> u8 {
		match self {
			Self::Array => b'[',
			Self::Map => b'{',
		}
	}
	fn get_end(&self) -> u8 {
		match self {
			Self::Array => b']',
			Self::Map => b'}',
		}
	}
}
#[derive(PartialEq, Debug)]
enum Delimiter {
	Stop,
	Item(u8),
	End(StructType),
	Skip,
	Start(StructType),
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
enum State {
	ParseObjectKey,
	ParseObject,
	ParseValue(ParseValueType),
	ExpectValue,
	/// We expect ":" or whitespace
	ExpectPoints,
	None,
}

const DEBUG: bool = false;

// we split the prefix at the beginning into sub parts

/// will only support the stream loading of an array of object under a object key chain, like "a.b.c"
/// does not support delimiting a struct of type array for now
fn iter_delimiters(
	iterator: impl Iterator<Item = u8> + 'static,
	prefix: Vec<u8>,
) -> impl Iterator<Item = Delimiter> + 'static {
	// in order to know where we are in the object
	let mut key_stack: Vec<Vec<u8>> = vec![];
	// the current key where we parse the value
	let mut current_key: Vec<u8> = vec![];

	// if we should be currently returning items
	let mut in_key = false;
	let mut object_nesting = 0;
	let mut array_nesting = 0;
	// if we have started returning values
	// can maybe optimized away later
	let mut started = false;
	// keeps state of the parsing
	let mut state = State::None;
	// prevents rebuilding the key stack without rebuilding it
	let mut stack_dirty = false;
	// Keeps state if the next character is escaped
	let mut escape = false;

	iterator
		.map(move |s| {
			let c = s;

			// not pretty
			if DEBUG {
				println!(
					"| {} | key {} | state {:?} | array nesting {}",
					c,
					compare_stack(&key_stack, &prefix),
					&state,
					array_nesting
				);
			}

			// avoid testing the key many times
			if stack_dirty && compare_stack(&key_stack, &prefix) {
				stack_dirty = false;
				in_key = true;
				// array_nesting = 1;
			}

			// if we are in the searched key
			// TODO: skip useless characters maybe
			if in_key {
				if c == b'[' {
					array_nesting += 1;
					if object_nesting == 0 && array_nesting == 2 {
						started = true;
						return Delimiter::Start(StructType::Array);
					}
				}

				if c == b']' {
					array_nesting -= 1;

					if object_nesting == 0 && array_nesting == 1 {
						started = false;
						return Delimiter::End(StructType::Array);
					}

					// end of parsing, skip the rest of the stream
					if array_nesting == 0 {
						in_key = false;
						return Delimiter::Stop;
					}
				}

				if c == b'{' {
					object_nesting += 1;
					if object_nesting == 1 && array_nesting == 1 {
						started = true;
						return Delimiter::Start(StructType::Map);
					}
				}

				if c == b'}' {
					object_nesting -= 1;
					if object_nesting == 0 && array_nesting == 1 {
						started = false;
						return Delimiter::End(StructType::Map);
					}
				}

				if started {
					return Delimiter::Item(c);
				} else {
					return Delimiter::Skip;
				}
			} else {
				stack_dirty = false;
			}

			// if escape char
			if c == b'\\' {
				escape = true;
				return Delimiter::Skip;
			}

			// here we search the key
			// should never return item, from this point on

			match &state {
				// handle current key count
				State::ParseObjectKey => {
					if c == b'\"' && !escape {
						state = State::ExpectPoints;
						// key_stack.push(current_key.);
						// todo: clean this
						key_stack.push(current_key.clone());
						current_key.clear();
						stack_dirty = true;
					} else {
						current_key.push(c);
					}
					return Delimiter::Skip;
				}
				State::ParseValue(t) => {
					// detect end of value
					match t {
						ParseValueType::String => {
							if c == b'\"' && !escape {
								state = State::ParseObject;
								key_stack.pop();
								stack_dirty = true;
							}
							if escape {
								escape = false;
							}
						}
						ParseValueType::Array => {
							if c == b']' {
								state = State::ParseObject;
								key_stack.pop();
								stack_dirty = true;
							}
						}
						ParseValueType::Number => {
							if c == b',' {
								state = State::ParseObject;
								key_stack.pop();
								stack_dirty = true;
							}
						}
						ParseValueType::Null => {
							if c == b',' {
								state = State::ParseObject;
								key_stack.pop();
								stack_dirty = true;
							}
						}
						ParseValueType::Map => {
							// key_stack.push(current_key.clone());
							// current_key.clear();
							state = State::ParseObject;
						}
					};
				}
				State::ExpectValue => {
					if c == b'\"' {
						state = State::ParseValue(ParseValueType::String);
					} else if c == b'n' {
						// speculative nominal value is null
						state = State::ParseValue(ParseValueType::Null);
					} else if c == b'{' {
						state = State::ParseValue(ParseValueType::Map);
					} else if c == b'[' {
						state = State::ParseValue(ParseValueType::Array);
					} else if c == b' ' {
					} else {
						state = State::ParseValue(ParseValueType::Number);
					}
				}
				State::ExpectPoints => {
					if c == b':' {
						state = State::ExpectValue;
					}
				}
				State::ParseObject => {
					if c == b'\"' {
						state = State::ParseObjectKey;
					}
					if c == b'}' {
						key_stack.pop();
						stack_dirty = true;
					}
				}
				State::None => {
					if c == b'{' {
						// start root of object
						state = State::ParseObject;
					} else if c == b'[' {
						todo!("arrays are unsupported for now");
					} else if c != b' ' {
						panic!("malformed");
					}
				}
			};
			Delimiter::Skip
		})
		.take_while(|e| *e != Delimiter::Stop)
}

/// Returns an iterator returning serde parsed struct when consumed
///
///
///
///
pub fn stream_read_items_at<T>(
	iterator: impl Iterator<Item = u8> + 'static,
	prefix: &str,
) -> impl Iterator<Item = serde_json::Result<T>>
where
	T: DeserializeOwned,
{
    let prepared_prefix = make_prefix(prefix);
	let r1 = iter_delimiters(iterator, prepared_prefix);

	fold_and_parse::<T>(r1)
}

