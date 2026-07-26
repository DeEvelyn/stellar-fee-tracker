use stellar_devkit::sandbox::environment::SandboxEnv;

#[test]
fn test_passing_assertions_do_not_panic() {
    let env = SandboxEnv::from_normal_fixture();

    // These should all pass without panicking
    env.assert_fee_in_range(0, 10000);
    env.assert_quality_score_above(0.5);
}

#[test]
#[should_panic(expected = "Fee out of range")]
fn test_failing_fee_range_assertion() {
    let env = SandboxEnv::from_normal_fixture();
    // This should panic because fees are not in range 0..1
    env.assert_fee_in_range(0, 1);
}

#[test]
#[should_panic(expected = "Quality score")]
fn test_failing_quality_score_assertion() {
    let env = SandboxEnv::from_normal_fixture();
    // This should panic because quality score is likely below 1.0
    env.assert_quality_score_above(1.0);
}
