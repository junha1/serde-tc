use serde_tc::*;

#[serde_tc(dispatcher, encoder, dict, tuple)]
trait Trait1 {
    fn f1(&self, a1: i64, a2: &str, a3: &i32) -> String;
    fn f2(&self) -> String;
    fn f3(&self, a1: i32);
}

#[serde_tc(dispatcher, encoder, dict, tuple, async_methods)]
trait Trait2: Sync {
    async fn f1(&self, a1: i64, a2: &str, a3: &i32) -> String;
    async fn f2(&self) -> String;
    async fn f3(&self, a1: i32);
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

#[async_trait::async_trait]
impl Trait2 for SimpleImpl {
    async fn f1(&self, a1: i64, a2: &str, a3: &i32) -> String {
        format!("{}{}{}", a1, a2, a3)
    }
    async fn f2(&self) -> String {
        "hi".to_owned()
    }
    async fn f3(&self, _a1: i32) {}
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
    let args = trait1_encoder_dict::f1(1, "hello", &3);
    assert_eq!(
        DispatchStringDict::dispatch(object_ref, "f1", &args).unwrap(),
        format!(r#""{}{}{}""#, 1, "hello", 3)
    );
}

#[tokio::test]
async fn test1_async() {
    let object = SimpleImpl;
    let object_ref = &object as &dyn Trait2;

    let args = trait2_encoder_tuple::f1(1, "hello", &3);
    assert_eq!(
        DispatchStringTupleAsync::dispatch(object_ref, "f1", &args)
            .await
            .unwrap(),
        format!(r#""{}{}{}""#, 1, "hello", 3)
    );
    let args = trait2_encoder_dict::f1(1, "hello", &3);
    assert_eq!(
        DispatchStringDictAsync::dispatch(object_ref, "f1", &args)
            .await
            .unwrap(),
        format!(r#""{}{}{}""#, 1, "hello", 3)
    );
}
