# JsonIT (Json Iterator)

If you want to read json items as a stream under a prefixed array in a json file

```rs
pub fn stream_read_items_at<T>(iterator: impl Iterator<Item = String> + 'static, prefix: String) -> impl Iterator<Item = serde_json::Result<T>>
where
    T: DeserializeOwned,

```

Should work like python ijson
