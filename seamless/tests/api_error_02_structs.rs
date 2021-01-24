use seamless::{ ApiError };

#[derive(ApiError)]
#[api_error(internal)]
struct Internal {
    error: String
}
impl std::fmt::Display for Internal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}

#[test]
fn test_internal() {
    let a = Internal { error: "hi".to_owned() };
    let e: ApiError = a.into();
    assert_eq!(e.internal_message, "hi".to_owned());
    assert_eq!(e.external_message, "Internal server error".to_owned());
    assert_eq!(e.code, 500);
}

#[derive(ApiError)]
#[api_error(external)]
struct External {
    error: String
}
impl std::fmt::Display for External {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}

#[test]
fn test_external() {
    let a = External { error: "hi".to_owned() };
    let e: ApiError = a.into();
    assert_eq!(e.internal_message, "hi".to_owned());
    assert_eq!(e.external_message, "hi".to_owned());
    assert_eq!(e.code, 500);
}

#[derive(ApiError)]
#[api_error(external = "Custom message")]
struct InternalWithMsg {
    error: String
}
impl std::fmt::Display for InternalWithMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}

#[test]
fn test_internal_with_message() {
    let a = InternalWithMsg { error: "hi".to_owned() };
    let e: ApiError = a.into();
    assert_eq!(e.internal_message, "hi".to_owned());
    assert_eq!(e.external_message, "Custom message".to_owned());
    assert_eq!(e.code, 500);
}

#[derive(ApiError)]
#[api_error(external = "Not Authed", code = 400)]
struct InternalWithMsgAndCode {
    error: String
}
impl std::fmt::Display for InternalWithMsgAndCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}

#[test]
fn test_internal_with_message_and_code() {
    let a = InternalWithMsgAndCode { error: "hi".to_owned() };
    let e: ApiError = a.into();
    assert_eq!(e.internal_message, "hi".to_owned());
    assert_eq!(e.external_message, "Not Authed".to_owned());
    assert_eq!(e.code, 400);
}