use super::*;

#[test]
fn build_status_serializes_as_development_only() {
    assert_eq!(
        serde_json::to_string(&DevelopmentBuildStatus::DevelopmentOnly).unwrap(),
        "\"development_only\""
    );
}
