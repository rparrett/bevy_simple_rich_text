use bevy::prelude::*;
use bevy_simple_rich_text::prelude::*;

fn main() {
    App::new()
        .register_type::<Rainbow>()
        .add_plugins((DefaultPlugins, RichTextPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, rainbow_text)
        .run();
}

#[derive(Component, Reflect)]
#[reflect(Component)]
struct Rainbow(i32);

fn setup(mut commands: Commands, mut styles: ResMut<StyleRegistry>) {
    commands.spawn(Camera2d::default());

    let font = TextFont {
        font_size: 40.,
        ..default()
    };

    styles.insert("white".to_string(), commands.spawn(font.clone()).id());
    styles.insert(
        "red".to_string(),
        commands
            .spawn((TextColor(Color::hsl(0., 0.9, 0.7)), font.clone()))
            .id(),
    );
    styles.insert(
        "blue".to_string(),
        commands
            .spawn((TextColor(Color::hsl(240., 0.9, 0.7)), font.clone()))
            .id(),
    );
    styles.insert(
        "rainbow".to_string(),
        commands
            .spawn((
                Rainbow(1),
                TextColor(Color::hsl(0., 0.9, 0.8)),
                font.clone(),
            ))
            .id(),
    );

    commands.spawn((
        RichText(
            "default[red]red[white]white[blue]blue[rainbow]rainbow[]default\n[[escaped]]"
                .to_string(),
        ),
        Node {
            align_self: AlignSelf::Center,
            justify_self: JustifySelf::Center,
            ..default()
        },
    ));
}

fn rainbow_text(
    mut query: Query<&mut TextColor, (With<Rainbow>, With<TextSpan>)>,
    time: Res<Time>,
) {
    for mut color in &mut query {
        color.0 = color.0.rotate_hue(time.delta_secs() * 180.);
    }
}
