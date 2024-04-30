use super::AuthorizationQueryParsingError as Error;
use url::Url;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RedirectUri(Url);

impl RedirectUri {
    pub fn new(uri: Url) -> Result<Self, Error> {
        if uri.fragment().is_some() {
            return Err(Error::InvalidUri);
        }
        if uri
            .query()
            .map(serde_urlencoded::from_str::<Vec<(String, String)>>)
            .transpose()
            .is_err()
        {
            return Err(Error::InvalidUri);
        }

        // TODO: Validate
        Ok(Self(uri))
    }

    pub fn get(&self) -> &Url {
        &self.0
    }
}
