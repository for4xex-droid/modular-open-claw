use oxilean_kernel::env::Environment;

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn test_environment_is_send_sync() {
    assert_send_sync::<Environment>();
}

#[test]
fn test_environment_construction() {
    let env = Environment::new();
    // Verify the environment was created (no panic during construction).
    drop(env);
}
