#[test]
fn compile_tests() {
    let t = trybuild::TestCases::new();

    /* api_body */

    t.pass("tests/api_body_compile_tests/01_struct.rs");
    t.compile_fail("tests/api_body_compile_tests/02_struct_no_serde_attrs.rs");
    t.pass("tests/api_body_compile_tests/03_enum.rs");

    t.compile_fail("tests/api_body_compile_tests/04_serialize_struct_only.rs");
    t.compile_fail("tests/api_body_compile_tests/05_deserialize_struct_only.rs");
    t.pass("tests/api_body_compile_tests/06_se_de_struct.rs");

    t.compile_fail("tests/api_body_compile_tests/07_serialize_enum_only.rs");
    t.compile_fail("tests/api_body_compile_tests/08_deserialize_enum_only.rs");
    t.pass("tests/api_body_compile_tests/09_se_de_enum.rs");

    t.pass("tests/api_body_compile_tests/10_flatten.rs");

    t.compile_fail("tests/api_body_compile_tests/11_enum_cant_mix_unit_named.rs");

    /* api_error */

    // Structs
    t.compile_fail("tests/api_error_compile_tests/01_basic.rs");
    t.pass("tests/api_error_compile_tests/02_basic_internal.rs");
    t.pass("tests/api_error_compile_tests/03_basic_external.rs");
    t.pass("tests/api_error_compile_tests/04_external_message.rs");
    t.pass("tests/api_error_compile_tests/05_msg_and_code.rs");
    t.compile_fail("tests/api_error_compile_tests/06_not_both.rs");
    t.pass("tests/api_error_compile_tests/07_delegate_to_inner.rs");

    // Enums
    t.compile_fail("tests/api_error_compile_tests/08_enum_basic.rs");
    t.pass("tests/api_error_compile_tests/09_enum_toplevel_attrs.rs");
    t.pass("tests/api_error_compile_tests/10_enum_fields.rs");
    t.compile_fail("tests/api_error_compile_tests/11_enum_empty.rs");
}
