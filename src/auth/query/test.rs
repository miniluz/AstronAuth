use super::{
    opaque_parameters::OpaqueParameters, redirect_uri::RedirectUri, response_type::ResponseType,
    AuthorizationQueryParams as Params, AuthorizationQueryParsingError as Error,
    AuthorizationRequestQuery, ScopeList,
};

fn parse_authorization_query(query: &str) -> Result<AuthorizationRequestQuery, Error> {
    query.parse()
}

const VALID_RESPONSE_TYPE_PARAM: (&str, &str) = ("response_type", "code");
const VALID_CLIENT_ID_PARAM: (&str, &str) = ("client_id", "valid_client_id");
const VALID_REDIRECT_URI_PARAM: (&str, &str) =
    ("redirect_uri", "https://example.org/foo/bar?hey=now");
const VALID_SCOPE_PARAM: (&str, &str) = ("scope", "scope_a scope_b");
const VALID_STATE_PARAM: (&str, &str) = ("state", "opaque");

fn valid_redirect_uri() -> RedirectUri {
    RedirectUri::new(url::Url::parse("https://example.org/foo/bar?hey=now").unwrap()).unwrap()
}

fn valid_scope() -> ScopeList {
    ScopeList::try_from("scope_a scope_b").unwrap()
}

#[test]
fn trivial_query() {
    let trivial_query = serde_urlencoded::to_string([
        VALID_RESPONSE_TYPE_PARAM,
        VALID_CLIENT_ID_PARAM,
        VALID_REDIRECT_URI_PARAM,
        VALID_SCOPE_PARAM,
    ])
    .unwrap();

    let trivial_auth_query = AuthorizationRequestQuery {
        opaque_parameters: OpaqueParameters(vec![]),
        response_type: ResponseType::new("code").unwrap(),
        client_id: VALID_CLIENT_ID_PARAM.1.to_owned(),
        redirect_uri: valid_redirect_uri(),
        scope: valid_scope(),
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
    let missing_response_type =
        serde_urlencoded::to_string([VALID_CLIENT_ID_PARAM, VALID_REDIRECT_URI_PARAM]).unwrap();

    assert_eq!(
        parse_authorization_query(&missing_response_type),
        Err(Error::MissingParameter(
            Params::ResponseType.name(),
            valid_redirect_uri()
        ))
    );

    // parameters without valuesmust be treated as unsent as per section 3.1. of RFC 6749
    let missing_response_type = serde_urlencoded::to_string([
        ("response_type", ""),
        VALID_CLIENT_ID_PARAM,
        VALID_REDIRECT_URI_PARAM,
    ])
    .unwrap();

    assert_eq!(
        parse_authorization_query(&missing_response_type),
        Err(Error::MissingParameter(
            Params::ResponseType.name(),
            valid_redirect_uri()
        ))
    );

    // missing client id
    let missing_client_id =
        serde_urlencoded::to_string([VALID_RESPONSE_TYPE_PARAM, VALID_REDIRECT_URI_PARAM]).unwrap();

    assert_eq!(
        parse_authorization_query(&missing_client_id),
        Err(Error::MissingParameter(
            Params::ClientId.name(),
            valid_redirect_uri()
        ))
    );

    let missing_client_id = serde_urlencoded::to_string([
        VALID_RESPONSE_TYPE_PARAM,
        ("client_id", ""),
        VALID_REDIRECT_URI_PARAM,
    ])
    .unwrap();

    assert_eq!(
        parse_authorization_query(&missing_client_id),
        Err(Error::MissingParameter(
            Params::ClientId.name(),
            valid_redirect_uri()
        ))
    );

    // missing redirect_uri
    let missing_redirect_uri =
        serde_urlencoded::to_string([VALID_RESPONSE_TYPE_PARAM, VALID_CLIENT_ID_PARAM]).unwrap();

    assert_eq!(
        parse_authorization_query(&missing_redirect_uri),
        Err(Error::InvalidUri)
    );

    let missing_redirect_uri = serde_urlencoded::to_string([
        VALID_RESPONSE_TYPE_PARAM,
        VALID_CLIENT_ID_PARAM,
        ("redirect_uri", ""),
    ])
    .unwrap();

    assert_eq!(
        parse_authorization_query(&missing_redirect_uri),
        Err(Error::InvalidUri)
    );
}

#[test]
fn invalid_parameters() {
    let invalid_redirect_uri = serde_urlencoded::to_string([
        VALID_RESPONSE_TYPE_PARAM,
        VALID_CLIENT_ID_PARAM,
        ("redirect_uri", "ht/tps://example.org/foo/bar?hey=now&test"),
        VALID_SCOPE_PARAM,
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
        VALID_RESPONSE_TYPE_PARAM,
        VALID_CLIENT_ID_PARAM,
        VALID_REDIRECT_URI_PARAM,
        ("scope", "invalid_scope_à"),
    ])
    .unwrap();

    assert_eq!(
        parse_authorization_query(&invalid_scope),
        Err(Error::InvalidScope(
            "invalid_scope_à".to_owned(),
            valid_redirect_uri()
        ))
    );
}

#[test]
fn ignore_opaque_parameters() {
    let repeated_opaque_params = serde_urlencoded::to_string([
        ("repeated1", "hey"),
        VALID_RESPONSE_TYPE_PARAM,
        ("repeated1", ""),
        VALID_CLIENT_ID_PARAM,
        ("repeated2", "once"),
        VALID_REDIRECT_URI_PARAM,
        VALID_SCOPE_PARAM,
        ("repeated2", "twice"),
        ("repeated2", "thrice"),
    ])
    .unwrap();

    let repeated_opaque_params_query = AuthorizationRequestQuery {
        opaque_parameters: OpaqueParameters(vec![
            ("repeated1".to_owned(), "hey".to_owned()),
            ("repeated1".to_owned(), "".to_owned()),
            ("repeated2".to_owned(), "once".to_owned()),
            ("repeated2".to_owned(), "twice".to_owned()),
            ("repeated2".to_owned(), "thrice".to_owned()),
        ]),
        response_type: ResponseType::new("code").unwrap(),
        client_id: "valid_client_id".to_owned(),
        redirect_uri: valid_redirect_uri(),
        scope: valid_scope(),
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
        VALID_RESPONSE_TYPE_PARAM,
        ("response_type", ""),
        VALID_CLIENT_ID_PARAM,
        VALID_REDIRECT_URI_PARAM,
        VALID_SCOPE_PARAM,
        VALID_STATE_PARAM,
    ])
    .unwrap();

    assert_eq!(
        parse_authorization_query(&repeated_response_type),
        Err(Error::RepeatedParameter(valid_redirect_uri()))
    );

    let repeated_client_id = serde_urlencoded::to_string([
        VALID_RESPONSE_TYPE_PARAM,
        VALID_CLIENT_ID_PARAM,
        ("client_id", ""),
        VALID_REDIRECT_URI_PARAM,
        VALID_SCOPE_PARAM,
        VALID_STATE_PARAM,
    ])
    .unwrap();

    assert_eq!(
        parse_authorization_query(&repeated_client_id),
        Err(Error::RepeatedParameter(valid_redirect_uri()))
    );

    let repeated_redirect_uri = serde_urlencoded::to_string([
        VALID_RESPONSE_TYPE_PARAM,
        VALID_CLIENT_ID_PARAM,
        VALID_REDIRECT_URI_PARAM,
        ("redirect_uri", ""),
        VALID_SCOPE_PARAM,
        VALID_STATE_PARAM,
    ])
    .unwrap();

    assert_eq!(
        parse_authorization_query(&repeated_redirect_uri),
        Err(Error::InvalidUri)
    );

    let repeated_scope = serde_urlencoded::to_string([
        VALID_RESPONSE_TYPE_PARAM,
        VALID_CLIENT_ID_PARAM,
        VALID_REDIRECT_URI_PARAM,
        VALID_SCOPE_PARAM,
        ("scope", ""),
        VALID_STATE_PARAM,
    ])
    .unwrap();

    assert_eq!(
        parse_authorization_query(&repeated_scope),
        Err(Error::RepeatedParameter(valid_redirect_uri()))
    );

    let repeated_state = serde_urlencoded::to_string([
        VALID_RESPONSE_TYPE_PARAM,
        VALID_CLIENT_ID_PARAM,
        VALID_REDIRECT_URI_PARAM,
        VALID_SCOPE_PARAM,
        VALID_STATE_PARAM,
        ("state", ""),
    ])
    .unwrap();

    assert_eq!(
        parse_authorization_query(&repeated_state),
        Err(Error::RepeatedParameter(valid_redirect_uri()))
    );
}

#[test]
fn unsupported_response_type() {
    let unsupported_response_type = serde_urlencoded::to_string([
        VALID_STATE_PARAM,
        ("response_type", "not_code"),
        VALID_CLIENT_ID_PARAM,
        VALID_REDIRECT_URI_PARAM,
        VALID_SCOPE_PARAM,
    ])
    .unwrap();

    assert_eq!(
        parse_authorization_query(&unsupported_response_type),
        Err(Error::UnsupportedResponseType(valid_redirect_uri()))
    );
}
