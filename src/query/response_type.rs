use super::AuthorizationQueryParsingError;

#[non_exhaustive]
#[derive(Debug, PartialEq, Eq)]
pub struct ResponseType;

impl ResponseType {
    pub fn new(response_type: &str) -> Result<Self, AuthorizationQueryParsingError> {
        if response_type == "code" {
            Ok(Self)
        } else {
            Err(AuthorizationQueryParsingError::UnsupportedResponseType)
        }
    }
}
