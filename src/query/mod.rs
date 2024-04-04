//! This module is necessary to handle the requirements the RFC has for preserving the query

use itertools::{Either, Itertools};
use poem::{
    error::ResponseError,
    http::{StatusCode, Uri},
    FromRequest, Request, Response,
};
use tracing::instrument;

#[cfg(test)]
mod test;

mod parsing;

trait QueryParams: std::str::FromStr {
    fn name(&self) -> &'static str;
    fn names() -> &'static [&'static str];
    fn variants() -> &'static [Self];

    fn split(key_value: (String, String)) -> Either<(String, String), (Self, String)> {
        match (&*key_value.0).parse::<Self>() {
            Err(_) => Either::Left((key_value.0, key_value.1)),
            Ok(param) => Either::Right((param, key_value.1)),
        }
    }
}

#[derive(Debug, PartialEq)]
enum AuthorizationQueryParams {
    ResponseType,
    ClientId,
    RedirectUri,
    Scope,
    State,
}

impl std::str::FromStr for AuthorizationQueryParams {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "response_type" => Ok(Self::ResponseType),
            "client_id" => Ok(Self::ClientId),
            "redirect_uri" => Ok(Self::RedirectUri),
            "scope" => Ok(Self::Scope),
            "state" => Ok(Self::State),
            _ => Err(()),
        }
    }
}

impl QueryParams for AuthorizationQueryParams {
    fn name(&self) -> &'static str {
        match self {
            Self::ResponseType => "response_type",
            Self::ClientId => "client_id",
            Self::RedirectUri => "redirect_uri",
            Self::Scope => "scope",
            Self::State => "state",
        }
    }

    fn names() -> &'static [&'static str] {
        &[
            "response_type",
            "client_id",
            "redirect_uri",
            "scope",
            "state",
        ]
    }

    fn variants() -> &'static [Self] {
        &[
            Self::ResponseType,
            Self::ClientId,
            Self::RedirectUri,
            Self::Scope,
            Self::State,
        ]
    }
}

pub type ResponseType = String;
pub type ClientId = String;
pub type RedirectUri = Uri;
pub type ScopeList = Vec<Scope>;
pub type State = String;

/// A scope is a valid scope according to
/// [section 3.3](https://datatracker.ietf.org/doc/html/rfc6749#section-3.3)
/// of the RFC
#[derive(PartialEq, Eq)]
pub struct Scope(String);

impl std::fmt::Debug for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
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

impl Into<String> for Scope {
    fn into(self) -> String {
        self.0
    }
}

/// Extracs all params not contained in non_opaque_keys.
/// Returns (opaque_parameters, remaining_parameters)
fn extract_opaque_parameters<T: QueryParams>(
    parameters: Vec<(String, String)>,
) -> (Vec<(String, String)>, Vec<(T, String)>) {
    return parameters
        .into_iter()
        .partition_map(|key_value| T::split(key_value));
}

/// Represents the authorization request query.
#[derive(Debug, PartialEq, Eq)]
pub struct AuthorizationRequestQuery {
    /// All the parameters that aren't client, redirect_uri, scope and state are preserved as-is.
    opaque_parameters: Vec<(String, String)>,
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
    #[error("{0}")]
    ParsingError(#[from] parsing::ParsingError),
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
            Self::MissingParameter(_) | Self::ParsingError(_) => "invalid_request",
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

        let (opaque_parameters, result) = parsing::parse_query::<Params, Error>(query)?;

        #[derive(Default, Debug)]
        struct OptionalSelf {
            response_type: Option<String>,
            client_id: Option<String>,
            redirect_uri: Option<String>,
            scope: Option<String>,
            state: Option<String>,
        }

        let optional_self = result.into_iter().fold(
            OptionalSelf::default(),
            |optional_self, (variant, optional_param)| match variant {
                Params::ResponseType => OptionalSelf {
                    response_type: optional_param,
                    ..optional_self
                },
                Params::ClientId => OptionalSelf {
                    client_id: optional_param,
                    ..optional_self
                },
                Params::RedirectUri => OptionalSelf {
                    redirect_uri: optional_param,
                    ..optional_self
                },
                Params::Scope => OptionalSelf {
                    scope: optional_param,
                    ..optional_self
                },
                Params::State => OptionalSelf {
                    state: optional_param,
                    ..optional_self
                },
            },
        );

        tracing::trace!("Finished parsing query: {:?}", optional_self);

        match optional_self.response_type {
            None => return Err(Error::MissingParameter(Params::ResponseType.name())),
            Some(s) if s != "code" => {
                return Err(Error::UnsupportedResponseType);
            }
            // Some and s == code
            _ => {}
        }

        let client_id = match optional_self.client_id {
            Some(client_id) => client_id,
            None => return Err(Error::MissingParameter(Params::ClientId.name())),
        };

        let redirect_uri = match optional_self.redirect_uri {
            Some(redirect_uri) => redirect_uri,
            None => return Err(Error::MissingParameter(Params::RedirectUri.name())),
        };
        let redirect_uri = Uri::from_str(&redirect_uri).map_err(|_err| Error::InvalidUri)?;

        let scope = optional_self
            .scope
            .unwrap_or_default()
            .split(' ')
            .map(Scope::try_from)
            .collect::<Result<_, _>>()?;

        let result = AuthorizationRequestQuery {
            opaque_parameters,
            client_id,
            redirect_uri,
            scope: scope,
            state: optional_self.state,
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
