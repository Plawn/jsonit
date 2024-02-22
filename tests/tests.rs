#[cfg(test)]
mod tests {
	use std::io::Read;
use std::{fs::File, io::BufReader};

	use std::sync::Once;
	

	static INIT: Once = Once::new();
	use jsonit::{stream_read_items_at, JsonSeqIterator, ReaderIter, make_path};
	use serde::de::DeserializeOwned;
	use serde::Deserialize;

	/// in order to ensure retest when the json test file changes
	macro_rules! build_on {
		($file:literal) => {
			const _: &[u8] = include_bytes!($file);
		};
	}

	#[derive(Deserialize, Debug)]
	struct Value {
		name: String,
		op: Vec<Op>,
	}
	#[derive(Deserialize, Debug)]
	struct Op {
		a: String,
	}

	build_on!("test.json");

	fn init_logging(level: log::LevelFilter) -> Result<(), fern::InitError> {
		let colors = fern::colors::ColoredLevelConfig::default().info(fern::colors::Color::Blue);
		fern::Dispatch::new()
			.format(move |out, message, record| {
				out.finish(format_args!(
					"{}[{}][{}] {message}",
					chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
					record.target(),
					colors.color(record.level()),
				))
			})
			.level(log::LevelFilter::Debug)
			.level_for(env!("CARGO_PKG_NAME"), level)
			.chain(std::io::stdout())
			.apply()?;
		Ok(())
	}

	fn load_as_chars() -> impl Iterator<Item = u8> {
		let f = File::open("./tests/test.json").expect("failed to read test file");
		let b = BufReader::new(f);
		let reader = ReaderIter::new(b);
		reader.map(|e| e.expect("failed to read file"))
	}

	#[test]
	fn test_nominal() {
		let prefix = "root.items";
		let mut count = 0;
		let chars = load_as_chars();
		for (index, i) in stream_read_items_at::<Value>(chars, prefix).enumerate() {
			match i {
				Ok(value) => {
					println!("{:?}", &value);
					count += 1;
					if index == 0 {
						assert!(value.name == "hello1");
						assert!(value.op.first().unwrap().a == "a");
					}
					if index == 1 {
						assert!(value.name == "hello2");
						assert!(value.op.first().unwrap().a == "a");
					}
				}
				Err(err) => {
					panic!("Failed to parse item: {}", err);
				}
			}
		}
		assert!(count == 2);
	}

	#[test]
	fn test_nominal_array() {
		let prefix = "array";
		let mut count = 0;
		let chars = load_as_chars();
		for (_, i) in stream_read_items_at::<Vec<u32>>(chars, prefix).enumerate() {
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

	use anyhow::Result as InternalResult;

	use log::info;

	fn setup_logging() {
		INIT.call_once(|| init_logging(log::LevelFilter::Debug).unwrap());
	}

	fn test_string_with_type_at<T: DeserializeOwned + std::fmt::Debug>(data: &str, at: &str) -> InternalResult<()> {
		setup_logging();
		let reader = data.as_bytes();
		let prefix  = make_path(at);
		// does not handle the number for the moment being
		let iterator = JsonSeqIterator::new(reader, &prefix);

		for res in iterator {
			let item: T = res?;
			info!("{:?}", item);
		}

		Ok(())
	}

	fn test_read_with_type_at<T: DeserializeOwned + std::fmt::Debug, R: Read, >(reader: R, at: &str) -> InternalResult<()> {
		setup_logging();
		let prefix  = make_path(at);
		// does not handle the number for the moment being
		let iterator = JsonSeqIterator::new(reader, &prefix);

		for res in iterator {
			let item: T = res?;
			info!("{:?}", item);
		}

		Ok(())
	}

	#[test]
	fn reader_number_option() -> InternalResult<()> {
		let data = r#"{"a": [ [1,2,null]] }"#;
		test_string_with_type_at::<Vec<Option<i32>>>(data, "a")
	}

	#[test]
	fn reader_struct() -> InternalResult<()> {
		#[derive(Debug, Deserialize)]
		struct S {
			_b: i32,
		}
		let data = r#"{"a": [{"_b": 1}, {"_b" : 2}]] }"#;
		test_string_with_type_at::<S>(data, "a")
	}

	#[test]
	fn reader_string_option() -> InternalResult<()> {
		let data = r#"{"a": [ "deb","sneb",null] }"#;
		test_string_with_type_at::<Option<String>>(data, "a")
	}

	#[test]
	fn reader_from_read() -> InternalResult<()> {
		test_read_with_type_at::<Value, _>(get_test_local_reader("./tests/test.json"), "root.items")
	}

	fn get_test_file(path: &str) -> File {
		let f: File = File::open(path).expect("failed to read test file");
		f
	}

	fn get_test_local_reader(path: &str) -> impl Read {
		let f = get_test_file(path);
		let reader = BufReader::new(f);
		reader
	}

	#[test]
	fn reader_from_read_empty() -> InternalResult<()> {
		test_read_with_type_at::<Value, _>(get_test_local_reader("./tests/test.json"), "empty")
	}

	#[test]
	fn reader_from_read_simple() -> InternalResult<()> {
		test_read_with_type_at::<Option<String>, _>(get_test_local_reader("./tests/simple.json"), "a")
	}
}
