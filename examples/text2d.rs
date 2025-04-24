//! An example showcasing rich text for 2d cameras in world-space.

use bevy::prelude::*;
use bevy_simple_rich_text::{prelude::*, StyleTag};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, RichTextPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        StyleTag::new("lg"),
        TextFont {
            font_size: 40.,
            ..default()
        },
    ));
    commands.spawn(RichText2d::new("[lg]Hello[]world"));
}
