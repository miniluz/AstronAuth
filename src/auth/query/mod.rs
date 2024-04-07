//! This module is necessary to handle the requirements the RFC has for preserving the query

use axum::extract::{FromRequest, Request};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use tracing::instrument;
use url::Url;

#[cfg(test)]
mod test;

mod opaque_parameters;
mod redirect_uri;
mod response_type;
mod scope;

use self::{opaque_parameters::OpaqueParameters, redirect_uri::RedirectUri};
use self::{response_type::ResponseType, scope::ScopeList};

#[derive(Debug, PartialEq)]
enum AuthorizationQueryParams {
    ResponseType,
    ClientId,
    RedirectUri,
    Scope,
    State,
}

impl AuthorizationQueryParams {
    fn name(&self) -> &'static str {
        match self {
            Self::ResponseType => "response_type",
            Self::ClientId => "client_id",
            Self::RedirectUri => "redirect_uri",
            Self::Scope => "scope",
            Self::State => "state",
        }
    }
}

pub type ClientId = String;
pub type State = String;

/// Represents the authorization request query.
#[derive(Debug, PartialEq, Eq)]
pub struct AuthorizationRequestQuery {
    /// All the parameters that aren't client, redirect_uri, scope and state are preserved as-is.
    opaque_parameters: OpaqueParameters,
    response_type: ResponseType,
    client_id: ClientId,
    redirect_uri: RedirectUri,
    scope: ScopeList,
    state: Option<State>,
}

#[derive(Debug, thiserror::Error, PartialEq)]
#[error("authorization query parsing error")]
pub enum AuthorizationQueryParsingError {
    #[error("only the \"code\" response_type is supported")]
    UnsupportedResponseType(RedirectUri),
    #[error("the query is not in urlencoded format")]
    ParsingError(RedirectUri),
    #[error("repeated parameter")]
    RepeatedParameter(RedirectUri),
    #[error("missing parameter {0:?}")]
    MissingParameter(&'static str, RedirectUri),
    #[error(
        "invalid scope format on `{0:?}`. check section 3.3 of RFC 6749 for the allowed characters"
    )]
    InvalidScope(String, RedirectUri),
    #[error("the redirect_uri is invalid")]
    InvalidUri,
}

impl AuthorizationQueryParsingError {
    fn standard_error_text(&self) -> &'static str {
        match self {
            Self::UnsupportedResponseType(_) => "unsupported_response_type",
            Self::MissingParameter(_, _) | Self::RepeatedParameter(_) | Self::ParsingError(_) => {
                "invalid_request"
            }
            Self::InvalidScope(_, _) => "invalid_scope",
            Self::InvalidUri => "server_error",
        }
    }
}

impl IntoResponse for AuthorizationQueryParsingError {
    fn into_response(self) -> Response {
        let standard_error_text = self.standard_error_text();
        let error_text = self.to_string();
        match self {
            Self::InvalidUri => (StatusCode::BAD_REQUEST, error_text).into_response(),
            Self::UnsupportedResponseType(redirect_uri)
            | Self::InvalidScope(_, redirect_uri)
            | Self::MissingParameter(_, redirect_uri)
            | Self::ParsingError(redirect_uri)
            | Self::RepeatedParameter(redirect_uri) => {
                let mut query: Vec<(String, String)> = match serde_urlencoded::from_str(
                    redirect_uri.get().query().unwrap_or_default(),
                ) {
                    Ok(vec) => vec,
                    Err(_) => return (StatusCode::BAD_REQUEST, error_text).into_response(),
                };
                query.push(("error".to_owned(), standard_error_text.to_owned()));

                let query = match serde_urlencoded::to_string(query) {
                    Ok(str) => str,
                    Err(_) => return (StatusCode::BAD_REQUEST, error_text).into_response(),
                };

                let mut redirect_uri = redirect_uri.get().clone();
                redirect_uri.set_query(Some(&query));

                (
                    StatusCode::SEE_OTHER,
                    [("Location", redirect_uri.to_string())],
                    error_text,
                )
                    .into_response()
            }
        }
    }
}

impl std::str::FromStr for AuthorizationRequestQuery {
    type Err = AuthorizationQueryParsingError;
    /// Tries to generate itself from a still percentage-encoded string.
    fn from_str(query: &str) -> Result<Self, Self::Err> {
        use AuthorizationQueryParams as Params;
        use AuthorizationQueryParsingError as Error;

        #[derive(Deserialize)]
        struct RedirectUriDeserializer {
            redirect_uri: String,
        }
        let redirect_uri = serde_urlencoded::from_str::<RedirectUriDeserializer>(query)
            .map_err(|_| Error::InvalidUri)?
            .redirect_uri;

        let redirect_uri = Url::parse(&redirect_uri).map_err(|_| Error::InvalidUri)?;
        let redirect_uri = RedirectUri::new(redirect_uri).map_err(|_| Error::InvalidUri)?;

        // All logic for rejecting URIs must go ABOVE HERE and must return InvalidUri.

        #[derive(Debug, Deserialize)]
        struct DeserializableSelf {
            response_type: Option<String>,
            client_id: Option<String>,
            redirect_uri: Option<String>,
            scope: Option<String>,
            state: Option<String>,
            #[serde(flatten)]
            opaque_parameters: OpaqueParameters,
        }

        impl DeserializableSelf {
            fn empty_to_none(self) -> Self {
                fn empty_to_none(option: Option<String>) -> Option<String> {
                    option.and_then(|s| if s == "" { None } else { Some(s) })
                }
                DeserializableSelf {
                    response_type: empty_to_none(self.response_type),
                    client_id: empty_to_none(self.client_id),
                    redirect_uri: self.redirect_uri,
                    scope: empty_to_none(self.scope),
                    state: empty_to_none(self.state),
                    opaque_parameters: self.opaque_parameters,
                }
            }
        }

        let deserializable_self = match serde_urlencoded::from_str::<DeserializableSelf>(query) {
            Ok(deserializable_self) => deserializable_self,
            Err(error) => {
                let error = if error.to_string().starts_with("duplicate field") {
                    Error::RepeatedParameter(redirect_uri)
                } else {
                    Error::ParsingError(redirect_uri)
                };
                return Err(error);
            }
        }
        .empty_to_none();

        tracing::trace!("Finished parsing query: {:?}", deserializable_self);

        let response_type = ResponseType::new(&deserializable_self.response_type.ok_or(
            Error::MissingParameter(Params::ResponseType.name(), redirect_uri.clone()),
        )?)
        .map_err(|_unsupported_response_type| {
            Error::UnsupportedResponseType(redirect_uri.clone())
        })?;

        let client_id = deserializable_self
            .client_id
            .ok_or(Error::MissingParameter(
                Params::ClientId.name(),
                redirect_uri.clone(),
            ))?;

        let scope = ScopeList::try_from(&deserializable_self.scope.unwrap_or_default() as &str)
            .map_err(|invalid_scope| Error::InvalidScope(invalid_scope.0, redirect_uri.clone()))?;

        let result = AuthorizationRequestQuery {
            opaque_parameters: deserializable_self.opaque_parameters,
            response_type,
            client_id,
            redirect_uri,
            scope,
            state: deserializable_self.state,
        };

        tracing::trace!("Resulted in: {:?}", result);

        Ok(result)
    }
}

/// Implementations for parsing
impl AuthorizationRequestQuery {
    /// Simply maps to `Self::try_from_query`
    #[instrument(name = "parse_authorization_query", skip_all)]
    async fn internal_from_request(req: Request) -> Result<Self, AuthorizationQueryParsingError> {
        // this string will be percent-encoded. we'll have to decode it!
        let query = req.uri().query().unwrap_or_default();

        tracing::trace!("Started to parse query: {:?}", query);

        let result = query.parse();

        tracing::trace!("Resulted in: {:?}", result);

        result
    }
}

#[async_trait::async_trait]
impl<S> FromRequest<S> for AuthorizationRequestQuery
where
    S: Send + Sync,
{
    type Rejection = AuthorizationQueryParsingError;
    /// Simply maps to `Self::internal_from_request`
    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        Self::internal_from_request(req).await.map_err(Into::into)
    }
}

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
                .description(Some("Response type. Determines which OAuth 2.0 flow will be followed. Since the only supported flow is \"code\", it must be set to \"code\""))
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
