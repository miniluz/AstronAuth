use crate::query::AuthorizationRequestQuery;
use poem_openapi::{payload::PlainText, OpenApi};

pub struct Api;

#[OpenApi]
impl Api {
    /// Hello world
    #[oai(path = "/", method = "get")]
    async fn index(&self, _test: AuthorizationRequestQuery) -> PlainText<&'static str> {
        PlainText("Hello World")
    }
}
