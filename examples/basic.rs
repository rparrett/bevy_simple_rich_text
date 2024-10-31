use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use bevy_simple_rich_text::{prelude::*, RegisteredStyle};

fn main() {
    App::new()
        .register_type::<Rainbow>()
        .add_plugins((DefaultPlugins, RichTextPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, rainbow_text)
        .add_systems(
            Update,
            change_default.run_if(input_just_pressed(KeyCode::Space)),
        )
        .run();
}

// TODO add an example of changing the default registered style

#[derive(Component, Reflect)]
#[reflect(Component)]
struct Rainbow;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());

    let font = TextFont {
        font_size: 40.,
        ..default()
    };

    commands.spawn((RegisteredStyle::new("white"), font.clone()));
    commands.spawn((
        RegisteredStyle::new("red"),
        TextColor(Color::hsl(0., 0.9, 0.7)),
        font.clone(),
    ));
    commands.spawn((
        RegisteredStyle::new("blue"),
        TextColor(Color::hsl(240., 0.9, 0.7)),
        font.clone(),
    ));
    commands.spawn((
        RegisteredStyle::new("rainbow"),
        Rainbow,
        TextColor(Color::hsl(0., 0.9, 0.8)),
        font.clone(),
    ));

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

fn change_default(
    mut commands: Commands,
    mut registry: ResMut<StyleRegistry>,
    style_query: Query<&TextColor>,
) {
    let default = registry.get_default();

    if style_query.get(*default).is_ok() {
        commands.entity(*default).remove::<TextColor>();
    } else {
        commands
            .entity(*default)
            .insert(TextColor::from(Srgba::gray(0.6)));
    }

    registry.set_changed();
}
