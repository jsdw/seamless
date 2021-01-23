
#[derive(Debug,Clone)]
pub struct ApiError {
    pub code: u16,
    pub internal_message: String,
    pub external_message: String,
    pub value: Option<serde_json::Value>
}

impl ApiError {

    // Try to keep this the same as seamless-macros::api_error's version:
    pub const SERVER_ERROR: &'static str = "Internal server error";

    pub fn server_error<S: Into<String>>(msg: S) -> ApiError {
        ApiError {
            code: 500,
            internal_message: msg.into(),
            external_message: ApiError::SERVER_ERROR.to_owned(),
            value: None
        }
    }

    pub fn path_not_found() -> ApiError {
        ApiError {
            code: 404,
            internal_message: "Not found".to_owned(),
            external_message: "Not found".to_owned(),
            value: None
        }
    }

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

pub trait IntoApiError {
    fn into_api_error(self) -> ApiError;
}

impl IntoApiError for ApiError {
    fn into_api_error(self) -> ApiError {
        self
    }
}

impl IntoApiError for std::convert::Infallible {
    fn into_api_error(self) -> ApiError {
        unreachable!()
    }
}
