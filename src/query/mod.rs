//! This module is necessary to handle the requirements the RFC has for preserving the query

use itertools::Itertools;
use poem::{
    error::ResponseError,
    http::{StatusCode, Uri},
    FromRequest, Request, Response,
};
use serde::Deserialize;
use tracing::instrument;

use self::opaque_parameters::OpaqueParameters;

#[cfg(test)]
mod test;

mod opaque_parameters;

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
pub type RedirectUri = Uri;
pub type State = String;

/// A scope is a valid scope according to
/// [section 3.3](https://datatracker.ietf.org/doc/html/rfc6749#section-3.3)
/// of the RFC
#[derive(PartialEq, Eq)]
pub struct Scope(String);
#[derive(Debug, PartialEq, Eq)]
pub struct ScopeList(pub Vec<Scope>);

impl std::fmt::Debug for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<Scope> for String {
    fn from(scope: Scope) -> Self {
        scope.0
    }
}

impl From<ScopeList> for String {
    fn from(scope_list: ScopeList) -> String {
        scope_list.0.into_iter().map(|scope| scope.0).join(" ")
    }
}

impl TryFrom<&str> for Scope {
    type Error = AuthorizationQueryParsingError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() == 0 {
            return Err(Self::Error::InvalidScope(value.to_owned()));
        }

        let all_valid = value.as_bytes().iter().all(|char| {
            *char == 0x21 || (0x23..=0x5B).contains(char) || (0x5D..=0x7E).contains(char)
        });

        if !(all_valid) {
            return Err(Self::Error::InvalidScope(value.to_owned()));
        }

        Ok(Self(value.to_owned()))
    }
}

impl TryFrom<&str> for ScopeList {
    type Error = AuthorizationQueryParsingError;

    fn try_from(values: &str) -> Result<Self, Self::Error> {
        values
            .split(' ')
            .map(Scope::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|list| ScopeList(list))
    }
}

/// Represents the authorization request query.
#[derive(Debug, PartialEq, Eq)]
pub struct AuthorizationRequestQuery {
    /// All the parameters that aren't client, redirect_uri, scope and state are preserved as-is.
    opaque_parameters: OpaqueParameters,
    pub client_id: ClientId,
    pub redirect_uri: RedirectUri,
    pub scope: ScopeList,
    pub state: Option<State>,
}

#[derive(Debug, thiserror::Error, PartialEq)]
#[error("authorization query parsing error")]
pub enum AuthorizationQueryParsingError {
    #[error("only the \"code\" response_type is supported")]
    UnsupportedResponseType,
    #[error("the query is not in urlencoded format")]
    ParsingError,
    #[error("repeated parameter")]
    RepeatedParameter,
    #[error("missing parameter {0:?}")]
    MissingParameter(&'static str),
    #[error(
        "invalid scope format on `{0:?}`. check section 3.3 of RFC 6749 for the allowed characters"
    )]
    InvalidScope(String),
    #[error("the redirect_uri is invalid")]
    InvalidUri,
}

impl AuthorizationQueryParsingError {
    fn standard_error_text(&self) -> &'static str {
        match self {
            Self::UnsupportedResponseType => "unsupported_response_type",
            Self::MissingParameter(_) | Self::RepeatedParameter | Self::ParsingError => {
                "invalid_request"
            }
            Self::InvalidScope(_) => "invalid_scope",
            Self::InvalidUri => "server_error",
        }
    }
}

impl ResponseError for AuthorizationQueryParsingError {
    // TODO: Implement actual statuses
    fn status(&self) -> StatusCode {
        match self {
            Self::MissingParameter(param)
                if *param == AuthorizationQueryParams::RedirectUri.name() =>
            {
                StatusCode::BAD_REQUEST
            }
            Self::InvalidUri => StatusCode::BAD_REQUEST,
            _ => StatusCode::SEE_OTHER,
        }
    }

    fn as_response(&self) -> Response {
        let status = self.status();
        let response_builder = Response::builder().status(status);
        let response_builder = match status {
            StatusCode::SEE_OTHER => response_builder.header("Location", "todo"),
            StatusCode::BAD_REQUEST | _ => response_builder,
        };
        response_builder.body(self.to_string())
    }
}

impl std::str::FromStr for AuthorizationRequestQuery {
    type Err = AuthorizationQueryParsingError;
    /// Tries to generate itself from a still percentage-encoded string.
    fn from_str(query: &str) -> Result<Self, Self::Err> {
        use AuthorizationQueryParams as Params;
        use AuthorizationQueryParsingError as Error;

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
                    redirect_uri: empty_to_none(self.redirect_uri),
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
                    Error::RepeatedParameter
                } else {
                    Error::ParsingError
                };
                return Err(error);
            }
        }
        .empty_to_none();

        tracing::trace!("Finished parsing query: {:?}", deserializable_self);

        match deserializable_self.response_type {
            None => return Err(Error::MissingParameter(Params::ResponseType.name())),
            Some(s) if s != "code" => {
                return Err(Error::UnsupportedResponseType);
            }
            // Some and s == code
            _ => {}
        }

        let client_id = match deserializable_self.client_id {
            Some(client_id) => client_id,
            None => return Err(Error::MissingParameter(Params::ClientId.name())),
        };

        let redirect_uri = match deserializable_self.redirect_uri {
            Some(redirect_uri) => redirect_uri,
            None => return Err(Error::MissingParameter(Params::RedirectUri.name())),
        };
        let redirect_uri = Uri::from_str(&redirect_uri).map_err(|_err| Error::InvalidUri)?;

        let scope = ScopeList::try_from(&deserializable_self.scope.unwrap_or_default() as &str)?;

        let result = AuthorizationRequestQuery {
            opaque_parameters: deserializable_self.opaque_parameters,
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
    async fn internal_from_request(req: &Request) -> Result<Self, AuthorizationQueryParsingError> {
        // this string will be percent-encoded. we'll have to decode it!
        let query = req.uri().query().unwrap_or_default();

        tracing::trace!("Started to parse query: {:?}", query);

        let result = query.parse();

        tracing::trace!("Resulted in: {:?}", result);

        result
    }
}

impl<'a> FromRequest<'a> for AuthorizationRequestQuery {
    /// Simply maps to `Self::internal_from_request`
    async fn from_request(req: &'a Request, _body: &mut poem::RequestBody) -> poem::Result<Self> {
        Self::internal_from_request(req).await.map_err(Into::into)
    }
}
