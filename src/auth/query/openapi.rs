use super::AuthorizationRequestQuery;

impl utoipa::IntoParams for AuthorizationRequestQuery {
    fn into_params(
        _parameter_in_provider: impl Fn() -> Option<utoipa::openapi::path::ParameterIn>,
    ) -> Vec<utoipa::openapi::path::Parameter> {
        use utoipa::openapi::path::{ParameterBuilder, ParameterIn};
        use utoipa::openapi::{KnownFormat, ObjectBuilder, Required, SchemaFormat, SchemaType};
        /*
            client_id: ClientId,
            redirect_uri: RedirectUri,
            scope: ScopeList,
            state: Option<State>,
            opaque_parameters: OpaqueParameters,
        */
        vec![
            ParameterBuilder::new()
                .name("response_type")
                .required(Required::True)
                .parameter_in(ParameterIn::Query)
                .description(Some("Response type. Determines which OAuth 2.0 flow will be followed. Since the only supported flow is \"code\", it must be set to \"code\"."))
                .example(Some(serde_json::from_str("\"code\"").unwrap()))
                .schema(Some(
                    ObjectBuilder::new()
                        .schema_type(SchemaType::String)
                ))
                .build(),
            ParameterBuilder::new()
                .name("client_id")
                .required(Required::True)
                .parameter_in(ParameterIn::Query)
                .description(Some("Client ID. Must have previously registered at the client registration endpoint."))
                .schema(Some(
                    ObjectBuilder::new()
                        .schema_type(SchemaType::String)
                ))
                .build(),
            ParameterBuilder::new()
                .name("redirect_uri")
                .required(Required::True)
                .parameter_in(ParameterIn::Query)
                .description(Some("URI to redirect to. Must match one previously registered by the client."))
                .schema(Some(
                    ObjectBuilder::new()
                        .schema_type(SchemaType::String)
                        .format(Some(SchemaFormat::KnownFormat(KnownFormat::Uri))))
                )
                .build(),
            ParameterBuilder::new()
                .name("scope")
                .required(Required::False)
                .parameter_in(ParameterIn::Query)
                .description(Some("Scopes to be requested from the user. A space-separated list formatted according to the [RFC](https://datatracker.ietf.org/doc/html/rfc6749#section-3.3)."))
                .schema(Some(
                    ObjectBuilder::new()
                        .schema_type(SchemaType::String)
                ))
                .build(),
            ParameterBuilder::new()
                .name("state")
                .required(Required::False)
                .parameter_in(ParameterIn::Query)
                .description(Some("Any string. Will be preserved exactly."))
                .schema(Some(
                    ObjectBuilder::new()
                        .schema_type(SchemaType::String)
                ))
                .build(),
        ]
    }
}
