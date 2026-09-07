use super::{requests_empty_window, requests_service_window};

#[test]
fn service_window_mode_is_explicit_and_exclusive() {
    assert_eq!(
        requests_service_window(&["--service-window".into()]),
        Ok(true)
    );
    assert!(
        requests_service_window(&["--service-window".into(), "--empty-window".into()]).is_err()
    );
    assert!(
        requests_service_window(&["--service-window".into(), "--server".into(), "dev".into()])
            .is_err()
    );
    assert_eq!(requests_service_window(&[]), Ok(false));
    assert_eq!(
        requests_service_window(&["--screen".into(), "inworld".into()]),
        Ok(false)
    );
    assert_eq!(
        requests_service_window(&["--empty-window".into()]),
        Ok(false)
    );
}

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
