use super::requests_empty_window;

#[test]
fn empty_window_mode_is_explicit_and_exclusive() {
    assert_eq!(requests_empty_window(&["--empty-window".into()]), Ok(true));
    assert!(
        requests_empty_window(&["--empty-window".into(), "--server".into(), "dev".into()]).is_err()
    );
    assert!(requests_empty_window(&["--empty-window".into(), "model.m2".into()]).is_err());
}

#[test]
fn normal_arguments_keep_normal_startup() {
    assert_eq!(requests_empty_window(&[]), Ok(false));
    assert_eq!(
        requests_empty_window(&["--screen".into(), "inworld".into()]),
        Ok(false)
    );
}
