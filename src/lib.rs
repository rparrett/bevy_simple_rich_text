//! A Bevy plugin the provides a simple rich text component.
//!
//! # Examples
//!
//! See the [examples](https://github.com/rparrett/bevy_simple_rich_text/tree/main/examples) folder.
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_simple_rich_text::{RichTextPlugin};
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
//!         RegisteredStyle::new("red"),
//!         TextColor(Color::hsl(0., 0.9, 0.7)),
//!     ));
//!     commands.spawn((RichText::new("[red]Text")));
//! }
//! ```

use std::iter;

use bevy::{
    app::{Plugin, Update},
    ecs::{
        component::{Component, ComponentId},
        entity::Entity,
        query::Changed,
        system::Resource,
        world::World,
    },
    hierarchy::DespawnRecursiveExt,
    prelude::{
        AppTypeRegistry, BuildChildren, Deref, DerefMut, DetectChanges, DetectChangesMut,
        FromWorld, IntoSystemConfigs, Mut, Or, Query, ReflectComponent, RemovedComponents, Res,
        ResMut, SystemSet, Text, Text2d, With,
    },
    text::TextSpan,
    utils::HashMap,
};

use parser::parse_richtext;

// TODO consider completely rebuilding style registry on any change
// TODO text2d

/// Commonly used types for `bevy_simple_rich_text`.
pub mod prelude {
    pub use crate::{RichText, RichText2d, RichTextPlugin, StyleRegistry};
}

mod parser;

/// The top-level component for rich text for `bevy_ui`.
#[derive(Component)]
#[require(Text)]
pub struct RichText(pub String);
impl RichText {
    /// Creates a new `RichText` with the provided markup.
    pub fn new(markup: impl Into<String>) -> Self {
        Self(markup.into())
    }
}

/// The top-level component for rich text in world-space for 2d cameras.
#[derive(Component)]
#[require(Text2d)]
pub struct RichText2d(pub String);
impl RichText2d {
    /// Creates a new `RichText` with the provided markup.
    pub fn new(markup: impl Into<String>) -> Self {
        Self(markup.into())
    }
}

/// A component marking an entity as a "registered style" that can be referred to
/// by its tag when defining a [`RichText`].
///
/// Intentionally not `Reflect` so that this doesn't end up on `TextSpan`s when
/// the style is cloned.
#[derive(Component)]
pub struct RegisteredStyle(String);
impl RegisteredStyle {
    /// Creates a new `RegisteredStyle` with the provided tag.
    pub fn new(tag: impl Into<String>) -> Self {
        Self(tag.into())
    }
}
impl Default for RegisteredStyle {
    fn default() -> Self {
        Self("".into())
    }
}

/// A `HashMap` containing a mapping of `RegisteredStyle` tags to the
/// `Entity`s holding their style components.
///
/// This `Resource` is automatically managed by `bevy_simple_rich_text`.
#[derive(Resource, Deref, DerefMut)]
pub struct StyleRegistry(pub HashMap<String, Entity>);

impl StyleRegistry {
    /// Gets the `Entity` holding the default style components (the
    /// [`RegisteredStyle`] with the tag `""`.)
    pub fn get_default(&self) -> &Entity {
        &self.0[""]
    }
    /// Gets the `Entity` holding the style components for `tag`, falling
    /// back to the `Entity` holding the default style components.
    pub fn get_or_default(&self, tag: &str) -> &Entity {
        self.0.get(tag).unwrap_or_else(|| self.get_default())
    }
}
impl FromWorld for StyleRegistry {
    fn from_world(world: &mut World) -> Self {
        Self(HashMap::from([(
            "".to_string(),
            world.spawn((DefaultStyle, RegisteredStyle::new(""))).id(),
        )]))
    }
}

/// A marker component for the [`RegisteredStyle`] that is associated with the
/// default style tag (`""`).
#[derive(Component)]
pub struct DefaultStyle;

/// A SystemSet containing the systems that process [`RichText`] and manage
/// [`StyleRegistry`].
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RichTextSet;

/// This plugin adds systems and initializes resources required for processing
/// [`RichText`].
pub struct RichTextPlugin;
impl Plugin for RichTextPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<StyleRegistry>();
        app.add_systems(
            Update,
            (richtext_changed, registry_changed, sync_registry).in_set(RichTextSet),
        );
    }
}

fn sync_registry(
    changed: Query<(Entity, &RegisteredStyle), Changed<RegisteredStyle>>,
    all: Query<(), With<RegisteredStyle>>,
    mut removed: RemovedComponents<RegisteredStyle>,
    mut registry: ResMut<StyleRegistry>,
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

fn registry_changed(registry: Res<StyleRegistry>, mut rt_query: Query<Mut<RichText>>) {
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

    world.resource_scope(|world, registry: Mut<StyleRegistry>| {
        for ent in ents {
            world.commands().entity(ent).despawn_descendants();
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

                    let components = {
                        let style_entt = world.entity(*style_ent);

                        let archetype = style_entt.archetype();
                        let components = archetype.components().collect::<Vec<_>>();
                        components
                    };

                    for component in components {
                        component_clone_via_reflect(world, component, *style_ent, span_ent);
                    }
                }
            }
        }
    });
}

fn component_clone_via_reflect(
    world: &mut World,
    component_id: ComponentId,
    source: Entity,
    target: Entity,
) {
    world.resource_scope::<AppTypeRegistry, ()>(|world, registry| {
        let registry = registry.read();

        let component_info = world
            .components()
            .get_info(component_id)
            .expect("Component must be registered");
        let Some(type_id) = component_info.type_id() else {
            return;
        };
        let Some(reflect_component) = registry.get_type_data::<ReflectComponent>(type_id) else {
            return;
        };
        let source_component = reflect_component
            .reflect(world.get_entity(source).expect("Source entity must exist"))
            .expect("Source entity must have reflected component")
            .clone_value();
        let mut target = world
            .get_entity_mut(target)
            .expect("Target entity must exist");
        reflect_component.apply_or_insert(&mut target, &*source_component, &registry);
    });
}
