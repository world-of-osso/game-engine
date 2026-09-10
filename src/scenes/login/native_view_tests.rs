use super::*;

struct Fixture {
    world: World,
    view: LoginView,
    form: LoginForm,
    artwork: ButtonArtwork,
    pieces: Vec<Entity>,
    label: Entity,
    borders: [Entity; 4],
    shade: Entity,
}

impl Fixture {
    fn new() -> Self {
        let mut world = World::new();
        let mut images = Assets::<Image>::default();
        let artwork = ButtonArtwork {
            states: std::array::from_fn(|state| {
                let image = images.add(Image::default());
                (0..9)
                    .map(|piece| ImageNode {
                        image: image.clone(),
                        rect: Some(Rect::new(
                            piece as f32,
                            state as f32,
                            piece as f32 + 1.0,
                            state as f32 + 1.0,
                        )),
                        ..default()
                    })
                    .collect()
            }),
            display_edges: [4.0; 4],
        };
        world.insert_resource(images);
        let root = world.spawn_empty().id();
        let camera = world.spawn_empty().id();
        let username_input = world.spawn_empty().id();
        let password_input = world.spawn_empty().id();
        let username_text = world.spawn(Text::new("old username")).id();
        let password_text = world.spawn(Text::new("old password")).id();
        let status_text = world.spawn(Text::new("old status")).id();
        let label = world
            .spawn((Text::new("Login"), TextColor(GOLD), LoginTint(GOLD)))
            .id();
        let pieces: Vec<_> = artwork.states[0]
            .iter()
            .map(|image| world.spawn((image.clone(), LoginTint(Color::WHITE))).id())
            .collect();
        let connect_button = world
            .spawn(ButtonVisual {
                pieces: pieces.clone(),
                label,
                artwork: artwork.clone(),
            })
            .id();
        let realm_button = world
            .spawn((
                Node {
                    display: Display::None,
                    ..default()
                },
                Visibility::Hidden,
            ))
            .id();
        let create_account_button = world.spawn_empty().id();
        let menu_button = world.spawn_empty().id();
        let exit_button = world.spawn_empty().id();
        let borders = std::array::from_fn(|index| {
            world
                .spawn((
                    ImageNode::default(),
                    InputBorder {
                        field: if index < 2 {
                            LoginFieldId::Username
                        } else {
                            LoginFieldId::Password
                        },
                        center: index % 2 == 0,
                    },
                ))
                .id()
        });
        let shade_color = Color::srgba(0.0, 0.0, 0.0, 0.22);
        let shade = world
            .spawn((BackgroundColor(shade_color), LoginTint(shade_color)))
            .id();
        Self {
            world,
            view: LoginView {
                root,
                camera,
                username_input,
                password_input,
                username_text,
                password_text,
                connect_button,
                realm_button,
                create_account_button,
                menu_button,
                exit_button,
                status_text,
            },
            form: LoginForm::default(),
            artwork,
            pieces,
            label,
            borders,
            shade,
        }
    }

    fn sync(
        &mut self,
        status: &str,
        focus: Option<LoginFieldId>,
        fade: f32,
        pressed: bool,
        hovered: bool,
    ) {
        sync_login_view(
            &mut self.world,
            &self.view,
            LoginViewState {
                form: &self.form,
                status,
                focus,
                fade,
                pressed: pressed.then_some(self.view.connect_button),
                hovered: hovered.then_some(self.view.connect_button),
            },
        );
    }
}

#[test]
fn status_is_replaced_cleared_and_unchanged_text_is_not_marked_changed() {
    let mut fixture = Fixture::new();
    fixture.form.username.set_text("Alessio");
    fixture.sync("Please fill in all fields", None, 1.0, false, false);
    assert_eq!(
        fixture
            .world
            .get::<Text>(fixture.view.status_text)
            .unwrap()
            .0,
        "Please fill in all fields"
    );
    fixture.world.clear_trackers();
    fixture.sync("Please fill in all fields", None, 1.0, false, false);
    for entity in [
        fixture.view.status_text,
        fixture.view.username_text,
        fixture.view.password_text,
    ] {
        assert!(
            !fixture
                .world
                .entity(entity)
                .get_ref::<Text>()
                .unwrap()
                .is_changed()
        );
    }
    fixture.sync("Connecting...", None, 1.0, false, false);
    assert_eq!(
        fixture
            .world
            .get::<Text>(fixture.view.status_text)
            .unwrap()
            .0,
        "Connecting..."
    );
    fixture.sync("", None, 1.0, false, false);
    assert_eq!(
        fixture
            .world
            .get::<Text>(fixture.view.status_text)
            .unwrap()
            .0,
        ""
    );
}

#[test]
fn password_presentation_masks_utf8_bytes_without_changing_credentials() {
    let mut fixture = Fixture::new();
    fixture.form.username.set_text("Alessio");
    fixture.form.password.set_text("é猫!");
    fixture.sync("", None, 1.0, false, false);
    assert_eq!(
        fixture
            .world
            .get::<Text>(fixture.view.username_text)
            .unwrap()
            .0,
        "Alessio"
    );
    assert_eq!(
        fixture
            .world
            .get::<Text>(fixture.view.password_text)
            .unwrap()
            .0,
        "******"
    );
    assert_eq!(fixture.form.password.text, "é猫!");
    fixture.form.password.set_text("");
    fixture.sync("", None, 1.0, false, false);
    assert_eq!(
        fixture
            .world
            .get::<Text>(fixture.view.password_text)
            .unwrap()
            .0,
        ""
    );
}

#[test]
fn focus_moves_gold_border_and_dark_fill_between_fields_and_clears() {
    let mut fixture = Fixture::new();
    let dark = Color::srgb(0.22, 0.16, 0.11);
    let lit = Color::srgb(0.32, 0.24, 0.16);
    let gold = Color::srgb(1.0, 0.78, 0.0);
    for (focus, expected) in [
        (
            Some(LoginFieldId::Username),
            [lit, gold, dark, Color::WHITE],
        ),
        (
            Some(LoginFieldId::Password),
            [dark, Color::WHITE, lit, gold],
        ),
        (None, [dark, Color::WHITE, dark, Color::WHITE]),
    ] {
        fixture.sync("", focus, 1.0, false, false);
        for (entity, color) in fixture.borders.into_iter().zip(expected) {
            assert_eq!(fixture.world.get::<ImageNode>(entity).unwrap().color, color);
        }
    }
}

#[test]
fn fade_scales_image_text_and_translucent_background_without_compounding() {
    let mut fixture = Fixture::new();
    for fade in [0.0, 0.5, 1.0, 0.5, 1.0] {
        fixture.sync("", None, fade, false, false);
        assert_eq!(
            fixture
                .world
                .get::<ImageNode>(fixture.pieces[0])
                .unwrap()
                .color
                .alpha(),
            fade
        );
        assert_eq!(
            fixture
                .world
                .get::<TextColor>(fixture.label)
                .unwrap()
                .0
                .alpha(),
            fade
        );
        let alpha = fixture
            .world
            .get::<BackgroundColor>(fixture.shade)
            .unwrap()
            .0
            .alpha();
        assert!((alpha - 0.22 * fade).abs() < 0.00001);
    }
}

#[test]
fn image_and_text_tints_preserve_transparent_node_backgrounds() {
    let mut fixture = Fixture::new();
    let image = fixture
        .world
        .spawn((
            Node::default(),
            ImageNode::default(),
            InputBorder {
                field: LoginFieldId::Username,
                center: false,
            },
        ))
        .id();
    let text = fixture
        .world
        .spawn((Node::default(), Text::new("Login"), LoginTint(GOLD)))
        .id();
    fixture
        .world
        .entity_mut(fixture.shade)
        .insert(Node::default());
    for entity in [image, text] {
        assert_eq!(
            fixture.world.get::<BackgroundColor>(entity).unwrap().0,
            Color::NONE
        );
    }
    for (focus, fade, tint) in [
        (
            Some(LoginFieldId::Username),
            1.0,
            Color::srgb(1.0, 0.78, 0.0),
        ),
        (Some(LoginFieldId::Password), 0.5, Color::WHITE),
        (None, 0.0, Color::WHITE),
        (
            Some(LoginFieldId::Username),
            1.0,
            Color::srgb(1.0, 0.78, 0.0),
        ),
    ] {
        fixture.sync("", focus, fade, false, false);
        assert_eq!(
            fixture.world.get::<ImageNode>(image).unwrap().color,
            tint.with_alpha(fade)
        );
        assert_eq!(
            fixture.world.get::<TextColor>(text).unwrap().0,
            GOLD.with_alpha(fade)
        );
        for entity in [image, text] {
            assert_eq!(
                fixture.world.get::<BackgroundColor>(entity).unwrap().0,
                Color::NONE
            );
        }
        assert_eq!(
            fixture
                .world
                .get::<BackgroundColor>(fixture.shade)
                .unwrap()
                .0,
            Color::srgba(0.0, 0.0, 0.0, 0.22 * fade)
        );
    }
}

#[test]
fn button_artwork_and_label_follow_hover_press_disabled_and_recovery() {
    let mut fixture = Fixture::new();
    for (status, pressed, hovered, index, color) in [
        ("", false, false, 0, GOLD),
        ("", false, true, 2, GOLD),
        ("", true, true, 1, Color::srgb(0.8, 0.65, 0.0)),
        ("Connecting...", true, true, 3, Color::srgb(0.5, 0.5, 0.5)),
        ("Please fill in all fields", false, false, 0, GOLD),
    ] {
        fixture.sync(status, None, 1.0, pressed, hovered);
        for (entity, expected) in fixture.pieces.iter().zip(&fixture.artwork.states[index]) {
            let actual = fixture.world.get::<ImageNode>(*entity).unwrap();
            assert_eq!(actual.image, expected.image);
            assert_eq!(actual.rect, expected.rect);
        }
        assert_eq!(
            fixture.world.get::<TextColor>(fixture.label).unwrap().0,
            color
        );
        assert_eq!(fixture.world.get::<Text>(fixture.label).unwrap().0, "Login");
    }
}

#[test]
fn presentation_updates_preserve_hidden_realm_control() {
    let mut fixture = Fixture::new();
    for (status, fade) in [("", 0.0), ("Connecting...", 0.5), ("", 1.0)] {
        fixture.sync(status, Some(LoginFieldId::Username), fade, true, true);
        assert_eq!(
            fixture
                .world
                .get::<Node>(fixture.view.realm_button)
                .unwrap()
                .display,
            Display::None
        );
        assert_eq!(
            *fixture
                .world
                .get::<Visibility>(fixture.view.realm_button)
                .unwrap(),
            Visibility::Hidden
        );
    }
}
