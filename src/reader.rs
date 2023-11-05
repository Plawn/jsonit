use {anyhow::Result as InternalResult, serde::de::DeserializeOwned, std::io::Read};

#[cfg(test)]
mod test {
    use anyhow::Result as InternalResult;

    use log::info;

    use crate::reader::{init_logging, JsonSeqIterator};


    #[test]
    fn reader() -> InternalResult<()> {
        init_logging(log::LevelFilter::Debug).unwrap();
    
        #[derive(Debug, serde_derive::Deserialize)]
        struct S {
            b: i32,
        }

        let reader = r#"{"a": ["deb", "ded"]}"#.as_bytes();
    
        // does not handle the number for the moment being
        let iterator = JsonSeqIterator::new(reader, ".a");
    
        for res in iterator {
            let item: String = res?;
            info!("{:?}", item);
        }
    
        Ok(())
    }
}



struct JsonSeqIterator<'a, R, O> {
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

	fn deserialize_one_item(&mut self) -> InternalResult<O> {
		O::deserialize(&mut serde_json::Deserializer::from_reader(&mut self.reader)).map_err(|e| e.into())
	}
}

impl<'a, R: Read, O: DeserializeOwned> Iterator for JsonSeqIterator<'_, R, O> {
	type Item = InternalResult<O>;
	fn next(&mut self) -> Option<Self::Item> {
		match self.state {
			State::NotStarted { path_to_look_for } => {
				// TODO advance the reader to the path. As a stub:
				loop {
					match self.next_char() {
						Err(e) => return Some(Err(e)),
						Ok(b'[') => {
							self.state = State::Started;
							return Some(self.deserialize_one_item());
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
							Some(self.deserialize_one_item())
						}
						w => {
							if w.is_ascii_whitespace() {
								continue;
							} else {
								Some(Err(anyhow::anyhow!("Unexpected character: {}", char::from(w))))
							}
						}
					},
				};
			},
			State::Ended => None,
		}
	}
}

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