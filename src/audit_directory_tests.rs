use super::*;

#[test]
fn existing_output_is_rejected_before_staging() {
    let directory = tempfile::tempdir().unwrap();
    let mut called = false;

    let error = publish_new_directory(directory.path(), |_| {
        called = true;
        Ok(())
    })
    .unwrap_err();

    assert!(!called);
    assert!(error.to_string().contains("refusing to overwrite"));
}

#[test]
fn failed_staging_does_not_publish_a_partial_directory() {
    let parent = tempfile::tempdir().unwrap();
    let output = parent.path().join("audit");

    let error =
        publish_new_directory::<()>(&output, |_| anyhow::bail!("write failed")).unwrap_err();

    assert!(error.to_string().contains("write failed"));
    assert!(!output.exists());
}
