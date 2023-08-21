#[cfg(not(miri))]
#[test]
fn trybuild_tests() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/trybuild_fail/*.rs");
}
