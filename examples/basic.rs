use bevy::prelude::*;
use bevy_simple_rich_text::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, RichTextPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    let font = TextFont {
        font_size: 40.,
        ..default()
    };

    let white = font.clone();
    let red = (TextColor(Color::hsl(0., 0.9, 0.7)), font.clone());
    let blue = (TextColor(Color::hsl(240., 0.9, 0.7)), font.clone());

    let style_registry = StyleRegistry::default().with_styles([
        ("red".to_string(), style_fn(move || red.clone())),
        ("white".to_string(), style_fn(move || white.clone())),
        ("blue".to_string(), style_fn(move || blue.clone())),
    ]);

    commands.insert_resource(style_registry);

    commands.spawn((
        Text::default(),
        RichText("default[red]red[white]white[blue]blue[]default\n[[escaped]]".to_string()),
        Style {
            align_self: AlignSelf::Center,
            justify_self: JustifySelf::Center,
            ..default()
        },
    ));
}
