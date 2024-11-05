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

    commands.spawn((RegisteredStyle::new("lg"), font.clone()));
    commands.spawn((
        RegisteredStyle::new("white"),
        TextColor(Color::hsl(0., 1.0, 1.0)),
    ));
    commands.spawn((
        RegisteredStyle::new("red"),
        TextColor(Color::hsl(0., 0.9, 0.7)),
    ));
    commands.spawn((
        RegisteredStyle::new("blue"),
        TextColor(Color::hsl(240., 0.9, 0.7)),
    ));
    commands.spawn((
        RegisteredStyle::new("rainbow"),
        Rainbow,
        TextColor(Color::hsl(0., 0.9, 0.8)),
    ));

    commands.spawn((
        RichText::new(concat!(
            "default[lg,red]red[lg,white]white[lg,blue]blue[lg,rainbow]rainbow[]default\n",
            "[[escaped brackets]]\n",
            "Press [rainbow]space[] to change default style."
        )),
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
    // TODO this system needs to run after the systems in bevy_simple_rich_text.
    // Otherwise when e.g. pressing space to change the defaults, this text will
    // briefly flash its initial color.
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
