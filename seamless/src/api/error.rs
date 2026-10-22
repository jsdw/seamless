/// This represents an API error that is returned from the API.
#[derive(Debug, Clone, PartialEq)]
pub struct ApiError {
    /// What the HTTP status code should be for this error response.
    pub code: u16,
    /// A message that can be logged internally but should not be shown to API consumers.
    pub internal_message: String,
    /// A message that can be shown to API consumers.
    pub external_message: String,
    /// Some optional context which could contain arbitrary information. It's expected that
    /// this could be handed back to API consumers and so shouldn't contain anything sensitive.
    pub value: Option<serde_json::Value>,
}

impl ApiError {
    // Also used as the default external message by the ApiError macro:
    #[doc(hidden)]
    pub const SERVER_ERROR: &'static str = "Internal server error";

    /// An error whose message is used as both the internal and the external message.
    ///
    /// **Note:** the message will be shown to API consumers, so be careful not to include
    /// anything sensitive in it. Use [`ApiError::server_error()`] for errors that should
    /// only be seen internally.
    pub fn external(code: u16, msg: impl Into<String>) -> ApiError {
        let msg = msg.into();
        ApiError {
            code,
            internal_message: msg.clone(),
            external_message: msg,
            value: None,
        }
    }

    /// A helper to instantiate a server error.
    pub fn server_error<S: Into<String>>(msg: S) -> ApiError {
        ApiError {
            code: 500,
            internal_message: msg.into(),
            external_message: ApiError::SERVER_ERROR.to_owned(),
            value: None,
        }
    }

    /// A helper to instantiate a 404 not found error.
    pub fn path_not_found() -> ApiError {
        ApiError::external(404, "Not found")
    }

    /// A helper to instantiate a not authorized error.
    pub fn not_authorized(reason: &str) -> ApiError {
        ApiError::external(403, format!("Not Authorized: {reason}"))
    }
}

impl From<std::convert::Infallible> for ApiError {
    fn from(never: std::convert::Infallible) -> ApiError {
        match never {}
    }
}
