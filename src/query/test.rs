use poem::http::Uri;

use super::{
    AuthorizationQueryParams as Params, AuthorizationQueryParsingError as Error,
    AuthorizationRequestQuery, QueryParams,
};

fn parse_authorization_query(query: &str) -> Result<AuthorizationRequestQuery, Error> {
    query.parse()
}

#[test]
fn trivial_query() {
    let trivial_query = serde_urlencoded::to_string([
        ("response_type", "code"),
        ("client_id", "valid_client_id"),
        ("redirect_uri", "https://example.org/foo/bar?hey=now&test"),
        ("scope", "scope_a scope_b"),
    ])
    .unwrap();

    let trivial_auth_query = AuthorizationRequestQuery {
        opaque_parameters: vec![],
        client_id: "valid_client_id".to_owned(),
        redirect_uri: Uri::from_static("https://example.org/foo/bar?hey=now&test"),
        scope: vec!["scope_a".try_into().unwrap(), "scope_b".try_into().unwrap()],
        state: None,
    };

    assert_eq!(
        parse_authorization_query(&trivial_query).unwrap(),
        trivial_auth_query
    );
}

#[test]
fn missing_parameters() {
    // missins response type
    let missing_response_type = serde_urlencoded::to_string([
        ("client_id", "valid_client_id"),
        ("redirect_uri", "https://example.org/foo/bar?hey=now&test"),
    ])
    .unwrap();

    assert_eq!(
        parse_authorization_query(&missing_response_type),
        Err(Error::MissingParameter(Params::ResponseType.name()))
    );

    // parameters without valuesmust be treated as unsent as per section 3.1. of RFC 6749
    let missing_response_type = serde_urlencoded::to_string([
        ("response_type", ""),
        ("client_id", "valid_client_id"),
        ("redirect_uri", "https://example.org/foo/bar?hey=now&test"),
    ])
    .unwrap();

    assert_eq!(
        parse_authorization_query(&missing_response_type),
        Err(Error::MissingParameter(Params::ResponseType.name()))
    );

    // missing client id
    let missing_client_id = serde_urlencoded::to_string([
        ("response_type", "code"),
        ("redirect_uri", "https://example.org/foo/bar?hey=now&test"),
    ])
    .unwrap();

    assert_eq!(
        parse_authorization_query(&missing_client_id),
        Err(Error::MissingParameter(Params::ClientId.name()))
    );

    let missing_client_id = serde_urlencoded::to_string([
        ("response_type", "code"),
        ("client_id", ""),
        ("redirect_uri", "https://example.org/foo/bar?hey=now&test"),
    ])
    .unwrap();

    assert_eq!(
        parse_authorization_query(&missing_client_id),
        Err(Error::MissingParameter(Params::ClientId.name()))
    );

    // missing redirect_uri
    let missing_redirect_uri =
        serde_urlencoded::to_string([("response_type", "code"), ("client_id", "valid_client_id")])
            .unwrap();

    assert_eq!(
        parse_authorization_query(&missing_redirect_uri),
        Err(Error::MissingParameter(Params::RedirectUri.name()))
    );

    let missing_redirect_uri = serde_urlencoded::to_string([
        ("response_type", "code"),
        ("client_id", "valid_client_id"),
        ("redirect_uri", ""),
    ])
    .unwrap();

    assert_eq!(
        parse_authorization_query(&missing_redirect_uri),
        Err(Error::MissingParameter(Params::RedirectUri.name()))
    );
}

#[test]
fn invalid_parameters() {
    let invalid_redirect_uri = serde_urlencoded::to_string([
        ("response_type", "code"),
        ("client_id", "valid_client_id"),
        ("redirect_uri", "ht/tps://example.org/foo/bar?hey=now&test"),
        ("scope", "scope_a scope_b"),
    ])
    .unwrap();

    assert_eq!(
        parse_authorization_query(&invalid_redirect_uri),
        Err(Error::InvalidUri)
    );
}

#[test]
fn invalid_scope() {
    let invalid_scope = serde_urlencoded::to_string([
        ("response_type", "code"),
        ("client_id", "valid_client_id"),
        ("redirect_uri", "https://example.org/foo/bar?hey=now&test"),
        ("scope", "invalid_scope_à"),
    ])
    .unwrap();

    assert_eq!(
        parse_authorization_query(&invalid_scope),
        Err(Error::InvalidScope("invalid_scope_à".to_owned()))
    );
}

#[test]
fn ignore_opaque_parameters() {
    let repeated_opaque_params = serde_urlencoded::to_string([
        ("repeated1", "hey"),
        ("response_type", "code"),
        ("repeated1", ""),
        ("client_id", "valid_client_id"),
        ("repeated2", "once"),
        ("redirect_uri", "https://example.org/foo/bar?hey=now&test"),
        ("scope", "scope_a scope_b"),
        ("repeated2", "twice"),
        ("repeated2", "thrice"),
    ])
    .unwrap();

    let repeated_opaque_params_query = AuthorizationRequestQuery {
        opaque_parameters: vec![
            ("repeated1".to_owned(), "hey".to_owned()),
            ("repeated1".to_owned(), "".to_owned()),
            ("repeated2".to_owned(), "once".to_owned()),
            ("repeated2".to_owned(), "twice".to_owned()),
            ("repeated2".to_owned(), "thrice".to_owned()),
        ],
        client_id: "valid_client_id".to_owned(),
        redirect_uri: Uri::from_static("https://example.org/foo/bar?hey=now&test"),
        scope: vec!["scope_a".try_into().unwrap(), "scope_b".try_into().unwrap()],
        state: None,
    };

    assert_eq!(
        parse_authorization_query(&repeated_opaque_params).unwrap(),
        repeated_opaque_params_query
    );
}

#[test]
fn repeated_parameters() {
    let repeated_response_type = serde_urlencoded::to_string([
        ("response_type", "code"),
        ("response_type", ""),
        ("client_id", "valid_client_id"),
        ("redirect_uri", "https://example.org/foo/bar?hey=now&test"),
        ("scope", "scope_a scope_b"),
        ("state", "opaque"),
    ])
    .unwrap();

    assert_eq!(
        parse_authorization_query(&repeated_response_type),
        Err(Error::RepeatedParameter(Params::ResponseType.name()))
    );

    let repeated_client_id = serde_urlencoded::to_string([
        ("response_type", "code"),
        ("client_id", "valid_client_id"),
        ("client_id", ""),
        ("redirect_uri", "https://example.org/foo/bar?hey=now&test"),
        ("scope", "scope_a scope_b"),
        ("state", "opaque"),
    ])
    .unwrap();

    assert_eq!(
        parse_authorization_query(&repeated_client_id),
        Err(Error::RepeatedParameter(Params::ClientId.name()))
    );

    let repeated_redirect_uri = serde_urlencoded::to_string([
        ("response_type", "code"),
        ("client_id", "valid_client_id"),
        ("redirect_uri", "https://example.org/foo/bar?hey=now&test"),
        ("redirect_uri", ""),
        ("scope", "scope_a scope_b"),
        ("state", "opaque"),
    ])
    .unwrap();

    assert_eq!(
        parse_authorization_query(&repeated_redirect_uri),
        Err(Error::RepeatedParameter(Params::RedirectUri.name()))
    );

    let repeated_scope = serde_urlencoded::to_string([
        ("response_type", "code"),
        ("client_id", "valid_client_id"),
        ("redirect_uri", "https://example.org/foo/bar?hey=now&test"),
        ("scope", "scope_a scope_b"),
        ("scope", ""),
        ("state", "opaque"),
    ])
    .unwrap();

    assert_eq!(
        parse_authorization_query(&repeated_scope),
        Err(Error::RepeatedParameter(Params::Scope.name()))
    );

    let repeated_state = serde_urlencoded::to_string([
        ("response_type", "code"),
        ("client_id", "valid_client_id"),
        ("redirect_uri", "https://example.org/foo/bar?hey=now&test"),
        ("scope", "scope_a scope_b"),
        ("state", "opaque"),
        ("state", ""),
    ])
    .unwrap();

    assert_eq!(
        parse_authorization_query(&repeated_state),
        Err(Error::RepeatedParameter(Params::State.name()))
    );
}

#[test]
fn unsupported_response_type() {
    let unsupported_response_type = serde_urlencoded::to_string([
        ("response_type", "not_code"),
        ("client_id", "valid_client_id"),
        ("redirect_uri", "https://example.org/foo/bar?hey=now&test"),
        ("scope", "scope_a scope_b"),
    ])
    .unwrap();

    assert_eq!(
        parse_authorization_query(&unsupported_response_type),
        Err(Error::UnsupportedResponseType)
    );
}
