use itertools::Itertools;

use super::QueryParams;

fn extract_opaque_parameters<T: QueryParams>(
    parameters: Vec<(String, String)>,
) -> (Vec<(String, String)>, Vec<(T, String)>) {
    return parameters
        .into_iter()
        .partition_map(|key_value| T::split(key_value));
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum ParsingError {
    #[error("query must be in application/x-www-form-urlencoded format")]
    ParsingError,
    #[error("parameter {0:?} sent more than once")]
    RepeatedParameter(&'static str),
    #[error("server error")]
    ServerError,
}

pub fn parse_query<Params: QueryParams + PartialEq + 'static, Error: From<ParsingError>>(
    query: &str,
) -> Result<(Vec<(String, String)>, Vec<(&Params, Option<String>)>), Error> {
    let parameters =
        serde_urlencoded::from_str(query).map_err(|_err| ParsingError::ParsingError)?;

    let (opaque_parameters, non_opaque_parameters) =
        extract_opaque_parameters::<Params>(parameters);

    enum OptionUndefined {
        Some(String),
        None,
        Undefined,
    }

    impl OptionUndefined {
        fn to_option(self) -> Option<String> {
            match self {
                Self::Some(s) => Some(s),
                Self::None | Self::Undefined => None,
            }
        }

        fn try_define(
            &mut self,
            value: String,
            param_name: &'static str,
        ) -> Result<(), ParsingError> {
            *self = match self {
                Self::Some(_) | Self::None => {
                    return Err(ParsingError::RepeatedParameter(param_name))
                }
                Self::Undefined => {
                    if value != "" {
                        Self::Some(value)
                    } else {
                        Self::None
                    }
                }
            };
            Ok(())
        }
    }

    let mut collect = Params::variants()
        .iter()
        .map(|variant| (variant, OptionUndefined::Undefined))
        .collect::<Vec<_>>();

    for (param, value) in non_opaque_parameters {
        let i = collect
            .iter()
            .position(|(variant, _)| param == **variant)
            // Should always work as long as QueryParams::variants returns all variants.
            .ok_or(ParsingError::ServerError)?;
        collect[i].1.try_define(value, param.name())?;
    }

    let result = collect
        .into_iter()
        .map(|(variant, option_undefined)| (variant, option_undefined.to_option()))
        .collect::<Vec<_>>();

    return Ok((opaque_parameters, result));
}
