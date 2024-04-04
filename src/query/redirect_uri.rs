use std::convert::Infallible;

use poem::http::Uri;

#[derive(Debug, PartialEq, Eq)]
pub struct RedirectUri(Uri);

impl RedirectUri {
    pub fn new(uri: Uri) -> Result<Self, Infallible> {
        // TODO: Validate
        Ok(Self(uri))
    }

    pub fn get(&self) -> &Uri {
        &self.0
    }
}
