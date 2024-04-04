use itertools::Itertools;

use super::AuthorizationQueryParsingError;
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
        scope_list
            .0
            .into_iter()
            .map(|scope| String::from(scope))
            .join(" ")
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
