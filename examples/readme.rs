//! An example showcasing rich text for bevy_ui.

use bevy::prelude::*;
use bevy_simple_rich_text::{StyleTag, prelude::*};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, RichTextPlugin))
        .add_systems(Startup, setup)
        .run();
}

#[derive(Component)]
struct FancyText;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Register style tags by spawning `StyleTag` with `TextFont`, `TextColor`,
    // and any other arbitrary Component.
    commands.spawn((
        StyleTag::new("lg"),
        TextFont {
            font_size: 40.,
            ..default()
        },
    ));
    commands.spawn((
        StyleTag::new("fancy"),
        TextColor(Color::hsl(0., 0.9, 0.7)),
        FancyText,
    ));

    // And use them
    commands.spawn(RichText::new("[lg]Hello [lg,fancy]World"));
}
