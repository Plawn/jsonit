use serde::de::DeserializeOwned;

fn fold_and_parse<T>(
    iterator: impl Iterator<Item = Delimiter> + 'static,
) -> impl Iterator<Item = serde_json::Result<T>>
where
    T: DeserializeOwned,
{
    let mut v: Vec<String> = vec![];

    iterator.filter_map(move |e| match e {
        Delimiter::Item(e) => {
            v.push(e);
            None
        }
        Delimiter::End => {
            v.push("}".into());
            // parse here
            let t = v.join("");
            Some(serde_json::from_str::<T>(&t))
        }
        Delimiter::Skip => None,
        Delimiter::Start => {
            v.clear();
            v.push("{".into());
            None
        }
    })
}

#[derive(PartialEq, Debug)]
pub enum Delimiter {
    Item(String),
    End,
    Skip,
    Start,
}

#[derive(PartialEq, Debug, Copy, Clone)]
enum ParseValueType {
    String,
    Number,
    Null,
    Undefined,
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
    // / for \" items
    // Escape,
}

fn stack_as_path(v: &Vec<String>) -> String {
    v.join(".")
}

const DEBUG: bool = false;

/// will only support the stream loading of an array of object under a object key chain, like "a.b.c"
/// c containing the objects of type T
fn iter_delimiters(
    iterator: impl Iterator<Item = String> + 'static,
    prefix: String,
) -> impl Iterator<Item = Delimiter> + 'static {
    let mut key_stack: Vec<String> = vec![];
    let mut current_key = String::new();

    let _in_key = false;
    let object_nesting = 0;
    let mut started = false;
    let mut state = State::None;
    // only handle when inside the returned value
    let array_nesting = 0;

    let mut escape = false;

    let mut parse_is_done = false;
    return iterator.map(move |s| {
        let c = s;

        // not pretty
        if DEBUG {
            println!(
                "| {} | key {} | state {:?}",
                c,
                stack_as_path(&key_stack),
                &state
            );
        }

        // return the rest as we are done with the iterator
        if parse_is_done {
            return Delimiter::Skip;
        }
        // if we are in the searched key
        if stack_as_path(&key_stack) == prefix {
            // end of parsing, skip the rest of the stream
            if c == "]" && array_nesting == 0 {
                parse_is_done = true;
                return Delimiter::Skip;
            } else {
                if c == "}" && object_nesting == 0 {
                    started = false;
                    return Delimiter::End;
                }
                if c == "{" && object_nesting == 0 {
                    started = true;
                    return Delimiter::Start;
                }
                if started {
                    return Delimiter::Item(c);
                } else {
                    return Delimiter::Skip;
                }
            }
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
                        }
                        if escape {
                            escape = false;
                        }
                    }
                    ParseValueType::Array => {
                        if c == "]" {
                            state = State::ParseObject;
                            key_stack.pop();
                        }
                    }
                    ParseValueType::Number => {
                        if c == "," {
                            state = State::ParseObject;
                            key_stack.pop();
                        }
                    }
                    ParseValueType::Null => {
                        if c == "," {
                            state = State::ParseObject;
                            key_stack.pop();
                        }
                    }
                    ParseValueType::Undefined => {
                        if c == "," {
                            state = State::ParseObject;
                            key_stack.pop();
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
                } else if c == "n" {
                    state = State::ParseValue(ParseValueType::Null);
                } else if c == "u" {
                    state = State::ParseValue(ParseValueType::Undefined);
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
                }
            }
            State::None => {
                if c == "{" {
                    // start root of object
                    state = State::ParseObject;
                } else if c == "[" {
                    panic!("arrays are unsupported for now");
                } else if c != " " {
                    panic!("malformed");
                }
            }
        };
        Delimiter::Skip
    });
    // here fold by start and end
    // parse result folded string
    // return stream of parsed struct
}

pub fn stream_read_items_at<T>(
    iterator: impl Iterator<Item = String> + 'static,
    prefix: String,
) -> impl Iterator<Item = serde_json::Result<T>>
where
    T: DeserializeOwned,
{
    let r1 = iter_delimiters(iterator, prefix);
    let r2 = fold_and_parse::<T>(r1);
    return r2;
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::BufRead, io::BufReader};

    use serde::Deserialize;

    use super::*;

    macro_rules! build_on {
        ($file:literal) => {
            const _: &[u8] = include_bytes!($file);
        };
    }

    #[derive(Deserialize, Debug)]
    struct V {
        name: String,
    }

    build_on!("test.json");

    #[test]
    fn it_works() {
        let prefix = "root.items";
        let f = File::open("./src/test.json").expect("failed to read test file");
        let reader = BufReader::new(f);
        // let reader = buffered(f, 10);
        let i = reader.lines().flat_map(|l| {
            l.unwrap()
                .chars()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
        });
        for (index, i) in stream_read_items_at::<V>(i, String::from(prefix)).enumerate() {
            match i {
                Ok(value) => {
                    if index == 0 {
                        assert!(value.name == "hello1");
                    }
                    if index == 1 {
                        assert!(value.name == "hello2");
                    }
                }
                Err(err) => {
                    panic!("Failed to parse item: {}", err);
                }
            }
        }
    }
}
