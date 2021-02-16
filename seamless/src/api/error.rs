/// This represents an API error that is returned from the API.
#[derive(Debug,Clone,PartialEq)]
pub struct ApiError {
    /// What the HTTP status code should be for this error response.
    pub code: u16,
    /// A message that can be logged internally but should not be shown to API consumers.
    pub internal_message: String,
    /// A message that can be shown to API consumers.
    pub external_message: String,
    /// Some optional context which could contain arbitrary information. It's expected that
    /// this could be handed back to API consumers and so shouldn't contain anything sensitive.
    pub value: Option<serde_json::Value>
}

impl ApiError {

    // Try to keep this the same as seamless-macros::api_error's version:
    #[doc(hidden)]
    pub const SERVER_ERROR: &'static str = "Internal server error";

    /// A helper to instantiate a server error.
    pub fn server_error<S: Into<String>>(msg: S) -> ApiError {
        ApiError {
            code: 500,
            internal_message: msg.into(),
            external_message: ApiError::SERVER_ERROR.to_owned(),
            value: None
        }
    }

    /// A helper to instantiate a 404 not found error.
    pub fn path_not_found() -> ApiError {
        ApiError {
            code: 404,
            internal_message: "Not found".to_owned(),
            external_message: "Not found".to_owned(),
            value: None
        }
    }

    /// A helper to instantiate a not authorized error.
    pub fn not_authorized(reason: &str) -> ApiError {
        let msg = format!("Not Authorized: {}", reason);
        ApiError {
            code: 403,
            external_message: msg.clone(),
            internal_message: msg,
            value: None
        }
    }
}

impl From<std::convert::Infallible> for ApiError {
    fn from(_: std::convert::Infallible) -> ApiError { unreachable!() }
}
