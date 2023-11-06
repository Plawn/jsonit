#[cfg(test)]
mod tests {
	use std::{fs::File, io::BufReader};

	use jsonit::{stream_read_items_at, ReaderIter, JsonSeqIterator};
	use serde::Deserialize;

	use super::*;

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

	
	#[test]
	fn reader() -> InternalResult<()> {
		init_logging(log::LevelFilter::Debug).unwrap();

		#[derive(Debug, serde_derive::Deserialize)]
		struct S {
			b: i32,
		}

		let reader = r#"{"a": [ [1,2,null]] }"#.as_bytes();

		// does not handle the number for the moment being
		let iterator = JsonSeqIterator::new(reader, ".a");

		for res in iterator {
			let item: Vec<Option<i32>> = res?;
			info!("{:?}", item);
		}

		Ok(())
	}

}
