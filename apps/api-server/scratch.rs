fn main() {
    let text = "{\"type\":\"createSurface\"} trailing";
    let mut iter = serde_json::Deserializer::from_str(text).into_iter::<serde_json::Value>();
    match iter.next() {
        Some(Ok(v)) => {
            println!("Got value: {}", v);
            println!("Offset: {}", iter.byte_offset());
        }
        Some(Err(e)) => println!("Error: {}", e),
        None => println!("None"),
    }
}
