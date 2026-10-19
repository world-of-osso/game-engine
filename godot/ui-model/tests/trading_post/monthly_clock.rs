use super::*;

fn timed_view(remaining: i64) -> View {
    let mut view = freeze_view();
    let mut data = (**view.snapshot.as_ref().unwrap()).clone();
    data.month_end_unix = data.now_unix + remaining;
    view.receive(Arc::new(data));
    view
}

#[test]
#[ignore = "TP_MONTHLY_CLOCK RED: Time column absent"]
fn trading_post_monthly_clock_time_column_counts_server_days_hours_minutes_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for (remaining, elapsed, expected) in [
            (172_860, 0, "2d"),
            (172_860, 86_461, "23h"),
            (3_660, 61, "59m"),
            (59, 0, "0m"),
        ] {
            let mut view = timed_view(remaining);
            view.refund_elapsed = elapsed;
            let authority = view.snapshot.clone();
            let registry = render(skin, view.clone());
            assert_eq!(
                text(&registry, "PerksProgramTimeRemaining5"),
                expected,
                "{skin:?}"
            );
            assert_eq!(
                rect(&registry, "PerksProgramTimeRemaining5")[2..],
                [42., 38.]
            );
            assert_eq!(
                view.snapshot, authority,
                "clock never edits server authority"
            );
            assert!(
                registry
                    .get_by_name("PerksProgramFrozenTimeRemaining4")
                    .is_none()
            );
        }
    }
}

#[test]
#[ignore = "TP_MONTHLY_CLOCK RED: Time column absent"]
fn trading_post_monthly_clock_time_column_red_threshold_purchased_dash_and_hover_cart() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = timed_view(3_601);
        let white = render(skin, view.clone());
        assert_eq!(
            font_color(&white, "PerksProgramTimeRemaining5"),
            [1., 1., 1., 1.]
        );
        view.refund_elapsed = 1;
        let red = render(skin, view.clone());
        assert_eq!(text(&red, "PerksProgramTimeRemaining5"), "1h");
        assert_eq!(
            font_color(&red, "PerksProgramTimeRemaining5"),
            [1., 0.1, 0.1, 1.]
        );
        view.product_hover = Some(5);
        let hovered = render(skin, view.clone());
        assert!(hovered.get_by_name("PerksProgramTimeRemaining5").is_none());
        assert!(hovered.get_by_name("PerksProgramCartToggle5").is_some());
        let mut purchased = (**view.snapshot.as_ref().unwrap()).clone();
        purchased
            .items
            .iter_mut()
            .find(|row| row.id == 5)
            .unwrap()
            .purchased = true;
        view.receive(Arc::new(purchased));
        let owned = render(skin, view);
        assert_eq!(text(&owned, "PerksProgramTimeRemaining5"), "--");
        assert_eq!(
            font_color(&owned, "PerksProgramTimeRemaining5"),
            [128. / 255., 128. / 255., 128. / 255., 1.]
        );
    }
}
