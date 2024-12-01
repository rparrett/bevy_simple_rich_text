//! An example showcasing rich text for bevy_ui.

use bevy::prelude::*;
use bevy_simple_rich_text::{prelude::*, StyleTag};

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
    // Start README content
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
    commands.spawn(RichText::new("[lg]Hello [lg,fancy]World"));
    // End README Content
}
