//! A Bevy plugin the provides a simple rich text component.
//!
//! # Examples
//!
//! See the [examples](https://github.com/rparrett/bevy_simple_rich_text/tree/main/examples) folder.
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_simple_rich_text::prelude::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(RichTextPlugin)
//!         .add_systems(Startup, setup)
//!         .run();
//! }
//!
//! fn setup(mut commands: Commands) {
//!     commands.spawn(Camera2d);
//!     commands.spawn((
//!         StyleTag::new("red"),
//!         TextColor(Color::hsl(0., 0.9, 0.7)),
//!     ));
//!     commands.spawn((RichText::new("[red]Text")));
//! }
//! ```

use std::iter;

use bevy::{
    app::{Plugin, Update},
    ecs::{
        component::Component, entity::Entity, hierarchy::Children, query::Changed, world::World,
    },
    platform_support::collections::HashMap,
    prelude::{
        Deref, DerefMut, DetectChanges, DetectChangesMut, FromWorld, IntoScheduleConfigs, Mut, Or,
        Query, RemovedComponents, Res, ResMut, Resource, SystemSet, Text, Text2d, With,
    },
    text::TextSpan,
};

use parser::parse_richtext;

/// Commonly used types for `bevy_simple_rich_text`.
pub mod prelude {
    pub use crate::{RichText, RichText2d, RichTextPlugin, StyleTag, StyleTags};
}

mod parser;

/// The top-level component for rich text for `bevy_ui`.
#[derive(Component)]
#[require(Text)]
pub struct RichText(pub String);
impl RichText {
    /// Creates a new [`RichText`] with the provided markup.
    pub fn new(markup: impl Into<String>) -> Self {
        Self(markup.into())
    }
}

/// The top-level component for rich text in world-space for 2d cameras.
#[derive(Component)]
#[require(Text2d)]
pub struct RichText2d(pub String);
impl RichText2d {
    /// Creates a new [`RichText2d`] with the provided markup.
    pub fn new(markup: impl Into<String>) -> Self {
        Self(markup.into())
    }
}

/// A component marking an entity as a "style tag" that can be referred to
/// by its inner string defining a [`RichText`].
///
/// Intentionally not `Reflect` so that this doesn't end up on `TextSpan`s when
/// the style is cloned.
#[derive(Component)]
pub struct StyleTag(pub String);
impl StyleTag {
    /// Creates a new `StyleTag` with the provided tag.
    pub fn new(tag: impl Into<String>) -> Self {
        Self(tag.into())
    }
}
impl Default for StyleTag {
    fn default() -> Self {
        Self("".into())
    }
}

/// A `HashMap` containing a mapping of `StyleTag` tags to the
/// `Entity`s holding their style components.
///
/// This `Resource` is automatically managed by `bevy_simple_rich_text`.
#[derive(Resource, Deref, DerefMut)]
pub struct StyleTags(pub HashMap<String, Entity>);

impl StyleTags {
    /// Gets the `Entity` holding the default style components (the
    /// [`StyleTag`] with the tag `""`.)
    pub fn get_default(&self) -> &Entity {
        &self.0[""]
    }
    /// Gets the `Entity` holding the style components for `tag`, falling
    /// back to the `Entity` holding the default style components.
    pub fn get_or_default(&self, tag: &str) -> &Entity {
        self.0.get(tag).unwrap_or_else(|| self.get_default())
    }
}
impl FromWorld for StyleTags {
    fn from_world(world: &mut World) -> Self {
        let mut map = HashMap::default();
        map.insert(
            "".to_string(),
            world.spawn((DefaultStyle, StyleTag::new(""))).id(),
        );
        Self(map)
    }
}

/// A marker component for the [`StyleTag`] that is associated with the
/// default style tag (`""`).
#[derive(Component)]
pub struct DefaultStyle;

/// A SystemSet containing the systems that process [`RichText`] and manage
/// [`StyleRegistry`].
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RichTextSystems;

/// This plugin adds systems and initializes resources required for processing
/// [`RichText`].
pub struct RichTextPlugin;
impl Plugin for RichTextPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<StyleTags>();
        app.add_systems(
            Update,
            (registry_changed, sync_registry, richtext_changed)
                .chain()
                .in_set(RichTextSystems),
        );
    }
}

fn sync_registry(
    changed: Query<(Entity, &StyleTag), Changed<StyleTag>>,
    all: Query<(), With<StyleTag>>,
    mut removed: RemovedComponents<StyleTag>,
    mut registry: ResMut<StyleTags>,
) {
    for ent in removed.read() {
        registry.0.retain(|_, v| *v != ent);
    }
    if changed.is_empty() {
        return;
    }
    for (ent, style) in &changed {
        registry.0.insert(style.0.clone(), ent);
    }

    registry.0.retain(|_, v| all.get(*v).is_ok());
}

fn registry_changed(registry: Res<StyleTags>, mut rt_query: Query<Mut<RichText>>) {
    if !registry.is_changed() {
        return;
    }

    for mut rt in &mut rt_query {
        rt.set_changed();
    }
}

fn richtext_changed(world: &mut World) {
    let mut ents_query =
        world.query_filtered::<Entity, Or<(Changed<RichText>, Changed<RichText2d>)>>();

    let ents = ents_query.iter(world).collect::<Vec<_>>();
    if ents.is_empty() {
        return;
    }

    let mut rt_query = world.query::<&RichText>();
    let mut rt_2d_query = world.query::<&RichText2d>();

    world.resource_scope(|world, registry: Mut<StyleTags>| {
        for ent in ents {
            world.commands().entity(ent).despawn_related::<Children>();
            world.flush();

            let Ok(rt) = rt_query
                .get(world, ent)
                .map(|rt| &rt.0)
                .or_else(|_| rt_2d_query.get(world, ent).map(|rt| &rt.0))
            else {
                continue;
            };

            let parsed = parse_richtext(rt);

            for section in parsed {
                let mut tags = vec!["".to_string()];
                tags.extend(section.tags);

                let span_ent = world.spawn(TextSpan::new(section.value.clone())).id();

                world.entity_mut(ent).add_child(span_ent);

                let empty_tags = iter::once("");
                for tag in empty_tags.chain(tags.iter().map(|t| t.as_str())) {
                    let style_ent = registry.get_or_default(tag);

                    world
                        .commands()
                        .entity(*style_ent)
                        .clone_with(span_ent, |builder| {
                            builder.deny::<(StyleTag, DefaultStyle)>();
                        });
                }
            }
        }
    });
}
