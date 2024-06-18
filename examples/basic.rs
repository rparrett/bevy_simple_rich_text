use bevy::prelude::*;
use bevy_simple_rich_text::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    let white = TextStyle {
        font_size: 40.,
        ..default()
    };

    let red = TextStyle {
        color: Color::hsl(0., 0.9, 0.7),
        ..white.clone()
    };
    let blue = TextStyle {
        color: Color::hsl(240., 0.9, 0.7),
        ..white.clone()
    };

    let style_registry = StyleRegistry::default().with_styles([
        ("red".to_string(), red),
        ("white".to_string(), white),
        ("blue".to_string(), blue),
    ]);

    commands.spawn(
        TextBundle::from_sections(rich(
            "default[red]red[white]white[blue]blue[]default\n[[escaped]]",
            &style_registry,
        ))
        .with_style(Style {
            align_self: AlignSelf::Center,
            justify_self: JustifySelf::Center,
            ..default()
        }),
    );
}
