//! This module is necessary to handle the requirements the RFC has for preserving the query

use poem::{
    error::ResponseError,
    http::{StatusCode, Uri},
    FromRequest, Request, Response,
};
use tracing::instrument;

#[cfg(test)]
mod test;

pub const RESPONSE_TYPE_PARAM: &str = "response_type";
pub type ResponseType = String;
pub const CLIENT_ID_PARAM: &str = "client_id";
pub type ClientId = String;
pub const REDIRECT_URI_PARAM: &str = "redirect_uri";
pub type RedirectUri = Uri;
pub const SCOPE_PARAM: &str = "scope";
pub type ScopeList = Vec<Scope>;
pub const STATE_PARAM: &str = "state";
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

    fn try_from(value: &str) -> std::prelude::v1::Result<Self, Self::Error> {
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
fn extract_opaque_parameters(
    parameters: Vec<(String, String)>,
    non_opaque_keys: &[&str],
) -> (Vec<(String, String)>, Vec<(String, String)>) {
    return parameters
        .into_iter()
        .partition(|(key, _value)| !non_opaque_keys.contains(&&**key));
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
    #[error("parameter {0:?} sent more than once")]
    RepeatedParameter(&'static str),
    #[error("query must be in application/x-www-form-urlencoded format")]
    ParsingError(#[from] serde_urlencoded::de::Error),
    #[error("missing parameter {0:?}")]
    MissingParameter(&'static str),
    #[error(
        "invalid scope format on `{0:?}`. check section 3.3 of RFC 6749 for the allowed characters"
    )]
    InvalidScope(String),
    // TODO: use
    #[error("the redirect_uri is invalid")]
    InvalidUri,
}

impl ResponseError for AuthorizationQueryParsingError {
    // TODO: Implement actual statuses
    fn status(&self) -> StatusCode {
        match self {
            Self::MissingParameter(REDIRECT_URI_PARAM) | Self::InvalidUri => {
                StatusCode::BAD_REQUEST
            }
            _ => StatusCode::SEE_OTHER,
        }
    }

    fn as_response(&self) -> Response {
        Response::builder()
            .status(self.status())
            .body(self.to_string())
    }
}

impl std::str::FromStr for AuthorizationRequestQuery {
    type Err = AuthorizationQueryParsingError;
    /// Tries to generate itself from a still percentage-encoded string.
    fn from_str(query: &str) -> Result<Self, Self::Err> {
        use AuthorizationQueryParsingError as Error;

        #[derive(Debug)]
        enum OptionUndefined<T> {
            Some(T),
            None,
            Undefined,
        }

        impl<T> OptionUndefined<T> {
            fn to_option(self) -> Option<T> {
                match self {
                    Self::Some(v) => Some(v),
                    Self::None | Self::Undefined => None,
                }
            }
        }

        impl OptionUndefined<String> {
            fn try_define(
                self,
                value: String,
                param_name: &'static str,
            ) -> std::result::Result<Self, Error> {
                match self {
                    Self::Some(_) | Self::None => Err(Error::RepeatedParameter(param_name)),
                    Self::Undefined => {
                        if value != "" {
                            Ok(Self::Some(value))
                        } else {
                            Ok(Self::None)
                        }
                    }
                }
            }
        }

        impl OptionUndefined<Vec<Scope>> {
            fn try_define(self, value: String) -> std::result::Result<Self, Error> {
                match self {
                    Self::Some(_) | Self::None => Err(Error::RepeatedParameter(SCOPE_PARAM)),
                    Self::Undefined => {
                        if value == "" {
                            Ok(Self::None)
                        } else {
                            Ok(Self::Some(
                                value
                                    .split(' ')
                                    .map(Scope::try_from)
                                    .collect::<Result<Vec<Scope>, Error>>()?,
                            ))
                        }
                    }
                }
            }
        }

        impl OptionUndefined<Uri> {
            fn try_define(self, value: String) -> std::result::Result<Self, Error> {
                match self {
                    Self::Some(_) | Self::None => Err(Error::RepeatedParameter(REDIRECT_URI_PARAM)),
                    Self::Undefined => {
                        if value == "" {
                            Ok(Self::None)
                        } else {
                            Ok(Self::Some(
                                value.parse::<Uri>().map_err(|_| Error::InvalidUri)?,
                            ))
                        }
                    }
                }
            }
        }

        let parameters =
            serde_urlencoded::from_str(query).map_err(|err| Error::ParsingError(err))?;

        let (opaque_parameters, non_opaque_parameters) = extract_opaque_parameters(
            parameters,
            &[
                RESPONSE_TYPE_PARAM,
                CLIENT_ID_PARAM,
                REDIRECT_URI_PARAM,
                SCOPE_PARAM,
                STATE_PARAM,
            ],
        );

        let mut response_type: OptionUndefined<ResponseType> = OptionUndefined::Undefined;
        let mut client_id: OptionUndefined<ClientId> = OptionUndefined::Undefined;
        let mut redirect_uri: OptionUndefined<RedirectUri> = OptionUndefined::Undefined;
        let mut scope: OptionUndefined<ScopeList> = OptionUndefined::Undefined;
        let mut state: OptionUndefined<State> = OptionUndefined::Undefined;

        for (key, value) in non_opaque_parameters {
            match &*key {
                RESPONSE_TYPE_PARAM => {
                    response_type = response_type.try_define(value, RESPONSE_TYPE_PARAM)?;
                }
                CLIENT_ID_PARAM => {
                    client_id = client_id.try_define(value, CLIENT_ID_PARAM)?;
                }
                REDIRECT_URI_PARAM => {
                    redirect_uri = redirect_uri.try_define(value)?;
                }
                SCOPE_PARAM => {
                    scope = scope.try_define(value)?;
                }
                STATE_PARAM => {
                    state = state.try_define(value, STATE_PARAM)?;
                }
                _ => { /* should be unreachable */ }
            }
        }

        tracing::trace!(
            concat!(
                "Finished parsing query: {{\n",
                "   opaque_parameters: {:?},\n",
                "   response_type: {:?},\n",
                "   client_id: {:?},\n",
                "   redirect_uri: {:?},\n",
                "   scope: {:?},\n",
                "   state: {:?},\n",
                "}}"
            ),
            opaque_parameters,
            response_type,
            client_id,
            redirect_uri,
            scope,
            state
        );

        match response_type.to_option() {
            None => return Err(Error::MissingParameter(RESPONSE_TYPE_PARAM)),
            Some(s) if s != "code" => {
                return Err(Error::UnsupportedResponseType);
            }
            // Some and s == code
            _ => {}
        }

        let client_id = match client_id.to_option() {
            Some(client_id) => client_id,
            None => return Err(Error::MissingParameter(CLIENT_ID_PARAM)),
        };

        let redirect_uri = match redirect_uri.to_option() {
            Some(redirect_uri) => redirect_uri,
            None => return Err(Error::MissingParameter(REDIRECT_URI_PARAM)),
        };

        let result = AuthorizationRequestQuery {
            opaque_parameters,
            client_id,
            redirect_uri,
            scope: scope.to_option().unwrap_or_default(),
            state: state.to_option(),
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
