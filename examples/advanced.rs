//! An example showing the basic functionality of `bevy_simple_rich_text`.

use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use bevy_simple_rich_text::{RichTextSystems, StyleTag, prelude::*};

fn main() {
    App::new()
        // Sibling components to `StyleTag` *must* be registered.
        .register_type::<Rainbow>()
        .add_plugins((DefaultPlugins, RichTextPlugin))
        .add_systems(Startup, setup)
        // `TextColor` or `TextFont` modifying systems should run after `RichTextSystems`
        // to prevent brief flashes of their tagged styles.
        .add_systems(Update, rainbow_text.after(RichTextSystems))
        .add_systems(
            Update,
            change_default.run_if(input_just_pressed(KeyCode::Space)),
        )
        .run();
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct Rainbow;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let font = TextFont {
        font_size: 40.,
        ..default()
    };

    commands.spawn((StyleTag::new("lg"), font.clone()));
    commands.spawn((StyleTag::new("white"), TextColor(Color::hsl(0., 1.0, 1.0))));
    commands.spawn((StyleTag::new("red"), TextColor(Color::hsl(0., 0.9, 0.7))));
    commands.spawn((StyleTag::new("blue"), TextColor(Color::hsl(240., 0.9, 0.7))));
    commands.spawn((
        StyleTag::new("rainbow"),
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
    for mut color in &mut query {
        color.0 = color.0.with_hue(time.elapsed_secs_wrapped() * 180. % 360.0);
    }
}

fn change_default(
    mut commands: Commands,
    mut registry: ResMut<StyleTags>,
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
