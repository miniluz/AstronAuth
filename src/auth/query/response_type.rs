#[non_exhaustive]
#[derive(Debug, PartialEq, Eq)]
pub struct ResponseType;

#[derive(Debug)]
pub struct UnsupportedResponseType;

impl ResponseType {
    pub fn new(response_type: &str) -> Result<Self, UnsupportedResponseType> {
        if response_type == "code" {
            Ok(Self)
        } else {
            Err(UnsupportedResponseType)
        }
    }
}
