use poem_openapi::{payload::PlainText, OpenApi};

pub struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/", method = "get")]
    async fn index(&self) -> PlainText<&'static str> {
        PlainText("hello world")
    }
}
