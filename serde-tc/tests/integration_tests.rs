use serde_json::Value;
use std::collections::HashMap;

#[test]
fn json_test() {
    let x = r#"{"James": [1, 2, 3], "John": [4, 5]}"#;
    let r: HashMap<String, String> = serde_json::from_str(&x).unwrap();
    println!("{:?}", r);
}
