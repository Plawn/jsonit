# JsonIT (Json Iterator)

This crate was created in order to make the streaming of Json Objects inside array in a ([std::Read] or [std::Iterator<u8>]) as easy as possible. It should ressemble the ijson package from python.

Like the ijson package you have to specify a prefix in order for the library to find the array you want to parse.


## Using with iterator

In order to parse an [std::Iterator<u8>] you can use this function

```rs
pub fn stream_read_items_at<T>(iterator: impl Iterator<Item = String> + 'static, prefix: String) -> impl Iterator<Item = serde_json::Result<T>>
where
    T: DeserializeOwned,

```

as per the example:


```rs
fn load_as_chars() -> impl Iterator<Item = u8> {
    let f = File::open("./tests/test.json").expect("failed to read test file");
    let b = BufReader::new(f);
    let reader = ReaderIter::new(b);
    reader.map(|e| e.expect("failed to read file"))
}
```

## Using with Read

as per the example:

```rs
// use ...
use jsonit::JsonItError;
use log::info;

type TestResult = Result<(), JsonItError>;

fn test_string_with_type_at<T: DeserializeOwned + std::fmt::Debug>(data: &str, at: &str) -> TestResult {
    setup_logging();
    let reader = data.as_bytes();
    let prefix = at.as_bytes();
    // does not handle the number for the moment being
    let iterator = JsonSeqIterator::new(reader, prefix);

    for res in iterator {
        let item: T = res?;
        info!("{:?}", item);
    }

    Ok(())
}

fn reader_number_option() -> TestResult {
    let data = r#"{"a": [ [1,2,null]] }"#;
    test_string_with_type_at::<Vec<Option<i32>>>(data, "a")
}
```