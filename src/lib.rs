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
        FromWorld, IntoSystemConfigs, Mut, Query, ReflectComponent, RemovedComponents, Res, ResMut,
        SystemSet, Text, With,
    },
    text::TextSpan,
    utils::HashMap,
};
use parser::parse_richtext;

// TODO consider completely rebuilding style registry on any change
// TODO text2d

pub mod prelude {
    pub use crate::RichText;
    pub use crate::RichTextPlugin;
    pub use crate::StyleRegistry;
}

mod parser;

#[derive(Component)]
#[require(Text)]
pub struct RichText(pub String);
impl RichText {
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

/// A component marking an entity as a "registered style" that can be refered to
/// by its tag when defining a [`RichText`].
///
/// Intentionally not `Reflect` so that this doesn't end up on `TextSpan`s when
/// the style is cloned.
#[derive(Component)]
pub struct RegisteredStyle(String);
impl RegisteredStyle {
    pub fn new(tag: impl Into<String>) -> Self {
        Self(tag.into())
    }
}
impl Default for RegisteredStyle {
    fn default() -> Self {
        Self("".into())
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct StyleRegistry(pub HashMap<String, Entity>);

impl StyleRegistry {
    pub fn get_default(&self) -> &Entity {
        &self.0[""]
    }
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
    let mut ents_query = world.query_filtered::<Entity, Changed<RichText>>();
    let mut rt_query = world.query::<&RichText>();

    let ents = ents_query.iter(world).collect::<Vec<_>>();
    if ents.is_empty() {
        return;
    }

    world.resource_scope(|world, registry: Mut<StyleRegistry>| {
        for ent in ents {
            world.commands().entity(ent).despawn_descendants();
            world.flush();

            let Ok(rt) = rt_query.get(world, ent) else {
                continue;
            };

            let parsed = parse_richtext(&rt.0);

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

pub fn component_clone_via_reflect(
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
