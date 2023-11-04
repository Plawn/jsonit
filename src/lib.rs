use serde::de::DeserializeOwned;

fn fold_and_parse<T>(iterator: impl Iterator<Item = Delimiter>) -> impl Iterator<Item = serde_json::Result<T>>
where
	T: DeserializeOwned,
{
	let mut v: Vec<String> = vec![];

	iterator.filter_map(move |e| match e {
		Delimiter::Item(e) => {
			v.push(e);
			None
		}
		Delimiter::End(e) => {
			v.push(e.get_end().to_owned());
			// parse here
			let t = v.join("");
			v.clear();
			println!("parsing: {}", &t);
			Some(serde_json::from_str::<T>(&t))
		}
		// should never arrive here
		Delimiter::Stop => panic!("Hum, we should never be here, got stop"),
		// should never arrive here
		Delimiter::Skip => None,
		Delimiter::Start(e) => {
			v.push(e.get_start().to_owned());
			None
		}
	})
}

#[derive(PartialEq, Debug)]
enum StructType {
    Map, Array
}

impl StructType {
    fn get_start(&self) -> &str {
        match self {
            Self::Array => "[",
            Self::Map => "{",
        }
    }
    fn get_end(&self) -> &str {
        match self {
            Self::Array => "]",
            Self::Map => "}",
        }
    }
}
#[derive(PartialEq, Debug)]
enum Delimiter {
	Stop,
	Item(String),
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

fn stack_as_path(v: &[String]) -> String {
	v.join(".")
}

const DEBUG: bool = false;

/// will only support the stream loading of an array of object under a object key chain, like "a.b.c"
/// does not support delimiting a struct of type array for now
fn iter_delimiters(
	iterator: impl Iterator<Item = String> + 'static,
	prefix: String,
) -> impl Iterator<Item = Delimiter> + 'static {
	// in order to know where we are in the object
    let mut key_stack: Vec<String> = vec![];
	// the current key where we parse the value
    let mut current_key = String::new();

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

	return iterator
        .map(move |s| {
            let c = s;

            // not pretty
            if DEBUG {
                println!(
                    "| {} | key {} | state {:?} | array nesting {}",
                    c,
                    stack_as_path(&key_stack),
                    &state,
                    array_nesting
                );
            }

            // avoid testing the key many times
            if stack_dirty && stack_as_path(&key_stack) == prefix {
                stack_dirty = false;
                in_key = true;
                // array_nesting = 1;
            }

            // if we are in the searched key
            // TODO: skip useless characters maybe
            if in_key {
                
                if c == "[" {
                    array_nesting += 1;
                    if object_nesting == 0 && array_nesting == 2 {
                        started = true;
                        return Delimiter::Start(StructType::Array);
                    }
                }

                if c == "]" {
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

                if c == "{"  {
                    object_nesting += 1;
                    if object_nesting == 1 && array_nesting == 1 {
                        started = true;
                        return Delimiter::Start(StructType::Map);
                    }
                }

                if c == "}" {
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
            if c == "\\" {
                escape = true;
                return Delimiter::Skip;
            }

            // here we search the key
            // should never return item, from this point on

            match &state {
                // handle current key count
                State::ParseObjectKey => {
                    if c == "\"" && !escape {
                        state = State::ExpectPoints;
                        key_stack.push(current_key.clone());
                        current_key.clear();
                        stack_dirty = true;
                    } else {
                        current_key.push_str(c.as_str());
                    }
                    return Delimiter::Skip;
                }
                State::ParseValue(t) => {
                    // detect end of value
                    match t {
                        ParseValueType::String => {
                            if c == "\"" && !escape {
                                state = State::ParseObject;
                                key_stack.pop();
                                stack_dirty = true;
                            }
                            if escape {
                                escape = false;
                            }
                        }
                        ParseValueType::Array => {
                            if c == "]" {
                                state = State::ParseObject;
                                key_stack.pop();
                                stack_dirty = true;
                            }
                        }
                        ParseValueType::Number => {
                            if c == "," {
                                state = State::ParseObject;
                                key_stack.pop();
                                stack_dirty = true;
                            }
                        }
                        ParseValueType::Null => {
                            if c == "," {
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
                    if c == "\"" {
                        state = State::ParseValue(ParseValueType::String);
                    } else if c == "n" { // speculative nominal value is null
                        state = State::ParseValue(ParseValueType::Null);
                    } else if c == "{" {
                        state = State::ParseValue(ParseValueType::Map);
                    } else if c == "[" {
                        state = State::ParseValue(ParseValueType::Array);
                    } else if c == " " {
                    } else {
                        state = State::ParseValue(ParseValueType::Number);
                    }
                }
                State::ExpectPoints => {
                    if c == ":" {
                        state = State::ExpectValue;
                    }
                }
                State::ParseObject => {
                    if c == "\"" {
                        state = State::ParseObjectKey;
                    }
                    if c == "}" {
                        key_stack.pop();
                        stack_dirty = true;
                    }
                }
                State::None => {
                    if c == "{" {
                        // start root of object
                        state = State::ParseObject;
                    } else if c == "[" {
                        todo!("arrays are unsupported for now");
                    } else if c != " " {
                        panic!("malformed");
                    }
                }
            };
            Delimiter::Skip
        })
        .take_while(|e| *e != Delimiter::Stop);
}


/// Returns an iterator returning serde parsed struct when consumed
/// 
/// 
/// 
/// 
pub fn stream_read_items_at<T>(
	iterator: impl Iterator<Item = String> + 'static,
	prefix: String,
) -> impl Iterator<Item = serde_json::Result<T>>
where
	T: DeserializeOwned,
{
	let r1 = iter_delimiters(iterator, prefix);

	fold_and_parse::<T>(r1)
}

#[cfg(test)]
mod tests {
	use std::{fs::File, io::BufRead, io::BufReader};

	use serde::Deserialize;

	use super::*;

    /// in order to ensure retest when the json test file changes
	macro_rules! build_on {
		($file:literal) => {
			const _: &[u8] = include_bytes!($file);
		};
	}

	#[derive(Deserialize, Debug)]
	struct V {
		name: String,
        op: Vec<Op>,
	}
    #[derive(Deserialize, Debug)]
    struct Op {
        a: String,
    }   

	build_on!("test.json");


    fn load_as_chars() -> impl Iterator<Item = String> {
		let f = File::open("./src/test.json").expect("failed to read test file");
		let reader = BufReader::new(f);
		// let reader = buffered(f, 10);
		let i = reader
			.lines()
			.flat_map(|l| l.unwrap().chars().map(|e| e.to_string()).collect::<Vec<_>>());
        i
    }

	#[test]
	fn test_nominal() {
        let prefix = "root.items";
		let mut count = 0;
        let chars = load_as_chars();
		for (index, i) in stream_read_items_at::<V>(chars, String::from(prefix)).enumerate() {
			match i {
				Ok(value) => {
					println!("{:?}", &value);
					count += 1;
					if index == 0 {
						assert!(value.name == "hello1");
                        assert!(value.op.get(0).unwrap().a == "a");
					}
					if index == 1 {
						assert!(value.name == "hello2");
                        assert!(value.op.get(0).unwrap().a == "a");
					}
				}
				Err(err) => {
					panic!("Failed to parse item: {}", err);
				}
			}
		}
		assert!(count == 2);
	}

    type Arr = Vec<u32>;

    #[test]
	fn test_nominal_array() {
        let prefix = "array";
		let mut count = 0;
        let chars = load_as_chars();
		for (_, i) in stream_read_items_at::<Arr>(chars, String::from(prefix)).enumerate() {
			match i {
				Ok(value) => {
					println!("{:?}", &value);
					count += 1;
				}
				Err(err) => {
					panic!("Failed to parse item: {}", err);
				}
			}
		}
		assert!(count == 8);
	}
}

