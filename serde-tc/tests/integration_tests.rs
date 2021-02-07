use serde_json::Value;
use serde_tc::*;
use std::collections::HashMap;

#[test]
fn json_test() {
    let x = r#"{"James": [1, 2, 3], "John": [4, 5]}"#;
    let r: HashMap<String, String> = serde_json::from_str(&x).unwrap();
    println!("{:?}", r);
}

#[serde_tc_str_debug]
trait Trait1 {
    fn f1(&self, a1: i64, a2: &str, a3: &i32) -> String;
    fn f2(&self) -> String;
    fn f3(&self, a1: i32);
}
