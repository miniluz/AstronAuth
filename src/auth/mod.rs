mod query;

pub use query::AuthorizationRequestQuery;

#[utoipa::path(
    get,
    path = "/authorization",
    params(AuthorizationRequestQuery),
    responses(
        (status = 303, description = "
Location will be set to request_uri.

If authorization is given, the `code`, the granted `scope` and the `state` parameter preserved as-is will be added to the Location's query string.

If an error occurs or authorization is not given, an `error` parameter with an explaination will be added to the query string"),
        (status = 400, description = "The request_uri is not valid or registered.")
    )
)]
pub async fn authorization(_test: AuthorizationRequestQuery) -> &'static str {
    "Hello World"
}
