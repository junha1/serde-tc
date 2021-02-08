use serde_tc::*;
use std::collections::HashMap;

#[test]
fn json_test() {
    let x = r#"{"James": [1, 2, 3], "John": [4, 5]}"#;
    let r: HashMap<String, String> = serde_json::from_str(&x).unwrap();
    println!("{:?}", r);
}

#[serde_tc(dispatcher, encoder, dict, tuple)]
trait Trait1 {
    fn f1(&self, a1: i64, a2: &str, a3: &i32) -> String;
    fn f2(&self) -> String;
    fn f3(&self, a1: i32);
}

struct SimpleImpl;

impl Trait1 for SimpleImpl {
    fn f1(&self, a1: i64, a2: &str, a3: &i32) -> String {
        format!("{}{}{}", a1, a2, a3)
    }

    fn f2(&self) -> String {
        "hi".to_owned()
    }

    fn f3(&self, _a1: i32) {}
}

#[test]
fn test1() {
    let object = SimpleImpl;
    let object_ref = &object as &dyn Trait1;

    let args = trait1_encoder_tuple::f1(1, "hello", &3);
    assert_eq!(
        DispatchStringTuple::dispatch(object_ref, "f1", &args).unwrap(),
        format!(r#""{}{}{}""#, 1, "hello", 3)
    );
}
