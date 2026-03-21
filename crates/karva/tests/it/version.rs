use insta_cmd::assert_cmd_snapshot;

use crate::common::TestContext;

#[test]
fn version_displays_version_info() {
    let context = TestContext::new();

    assert_cmd_snapshot!(context.version(), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    karva [VERSION]

    ----- stderr -----
    ");
}
