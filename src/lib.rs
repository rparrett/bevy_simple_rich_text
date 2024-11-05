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
use chumsky::{
    error::Cheap,
    primitive::{choice, just, none_of},
    Parser,
};

// TODO consider completely rebuilding style registry on any change
// TODO default style should be automatically composed into other specified styles

pub mod prelude {
    pub use crate::RichText;
    pub use crate::RichTextPlugin;
    pub use crate::StyleRegistry;
}

#[derive(Default)]

pub struct TextSection {
    value: String,
    tags: Vec<String>,
}

#[derive(Component)]
#[require(Text)]
pub struct RichText(pub String);
impl RichText {
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

// Added to entities holding reusable style components.
//
// Intentionally not `Reflect` so that this doesn't end up on `TextSpan`s when
// the style is cloned.
#[derive(Component)]
pub struct RegisteredStyle(String);
impl RegisteredStyle {
    pub fn new(tag: impl Into<String>) -> Self {
        Self(tag.into())
    }
    pub fn default() -> Self {
        Self("".into())
    }
}

#[derive(Component)]
pub struct DefaultStyle;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RichTextSet;

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

    // TODO lazy
    let empty_tags = vec!["".to_string()];

    world.resource_scope(|world, registry: Mut<StyleRegistry>| {
        for ent in ents {
            world.commands().entity(ent).despawn_descendants();
            world.flush();

            let Ok(rt) = rt_query.get(world, ent) else {
                continue;
            };

            let parsed = rich(&rt.0);

            for section in parsed {
                let tags = if section.tags.is_empty() {
                    &empty_tags
                } else {
                    &section.tags
                };

                let span_ent = world.spawn(TextSpan::new(section.value.clone())).id();

                world.entity_mut(ent).add_child(span_ent);

                for tag in tags {
                    let style_ent = registry.get_or_default(&tag);

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

#[derive(Resource, Deref, DerefMut)]
pub struct StyleRegistry(pub HashMap<String, Entity>);

impl<'a> StyleRegistry {
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

#[derive(Debug, PartialEq, Eq)]
enum TagsOrText {
    Tags(Vec<String>),
    Text(String),
}

fn escaped_bracket() -> impl Parser<char, String, Error = Cheap<char>> {
    just('[')
        .ignore_then(just('['))
        .or(just(']').ignore_then(just(']')))
        .map(|c| c.to_string())
}

fn tag_block() -> impl Parser<char, TagsOrText, Error = Cheap<char>> {
    tags()
        .delimited_by(just('['), just(']'))
        .map(TagsOrText::Tags)
}

fn tags() -> impl Parser<char, Vec<String>, Error = Cheap<char>> {
    not_end_bracket_or_comma()
        .separated_by(just(','))
        .collect::<Vec<_>>()
}

fn not_end_bracket_or_comma() -> impl Parser<char, String, Error = Cheap<char>> {
    none_of("],").repeated().at_least(1).collect::<String>()
}

fn not_any_bracket() -> impl Parser<char, String, Error = Cheap<char>> {
    none_of("[]").repeated().at_least(1).collect::<String>()
}

fn stray_end_bracket() -> impl Parser<char, String, Error = Cheap<char>> {
    just(']').map(|c| c.to_string())
}

fn text() -> impl Parser<char, TagsOrText, Error = Cheap<char>> {
    choice((escaped_bracket(), not_any_bracket(), stray_end_bracket()))
        .repeated()
        .at_least(1)
        .collect::<String>()
        .map(TagsOrText::Text)
}

fn tags_or_text() -> impl Parser<char, Vec<TagsOrText>, Error = Cheap<char>> {
    choice((text(), tag_block())).repeated().collect::<Vec<_>>()
}

pub fn rich(text: &str) -> Vec<TextSection> {
    let mut sections = vec![];
    let mut current_tags = vec![];

    let result = tags_or_text().parse(text);

    let tags_or_text = match result {
        Ok(tags_or_text) => tags_or_text,
        Err(errors) => {
            bevy::log::error!(
                "bevy_simple_rich_text failed to parse the input string. This should never happen."
            );
            bevy::log::error!("input: {}", text);
            for error in errors {
                bevy::log::error!(
                    "parsing failed at span {:?} with label {:?}",
                    error.span(),
                    error.label()
                );
            }

            sections.push(TextSection {
                value: "".to_string(),
                tags: current_tags,
            });

            return sections;
        }
    };

    for t in tags_or_text {
        match t {
            TagsOrText::Text(value) => sections.push(TextSection {
                value,
                tags: current_tags.clone(),
            }),
            TagsOrText::Tags(tags) => current_tags = tags,
        }
    }

    if sections.is_empty() {
        sections.push(TextSection {
            value: "".to_string(),
            tags: vec![],
        });
    }

    sections
}

#[test]
fn test_parser() {
    assert_eq!(
        tags_or_text().parse("[bold]"),
        Ok(vec![TagsOrText::Tags(vec!["bold".to_string()])])
    );
    assert_eq!(
        tags_or_text().parse("[[horse]]"),
        Ok(vec![TagsOrText::Text("[horse]".to_string())])
    );
    assert_eq!(
        tags_or_text().parse("[bold]Bold Text[italic]Italic Text"),
        Ok(vec![
            TagsOrText::Tags(vec!["bold".to_string()]),
            TagsOrText::Text("Bold Text".to_string()),
            TagsOrText::Tags(vec!["italic".to_string()]),
            TagsOrText::Text("Italic Text".to_string()),
        ])
    );
    assert_eq!(
        tags_or_text().parse("[]Text[]"),
        Ok(vec![
            TagsOrText::Tags(vec![]),
            TagsOrText::Text("Text".to_string()),
            TagsOrText::Tags(vec![]),
        ])
    );
    // escaping
    assert_eq!(
        tags_or_text().parse("[[]]][]"),
        Ok(vec![
            TagsOrText::Text("[]]".to_string()),
            TagsOrText::Tags(vec![]),
        ])
    );
    // multiple
    assert_eq!(
        tags_or_text().parse("[bold,italic]text"),
        Ok(vec![
            TagsOrText::Tags(vec!["bold".to_string(), "italic".to_string()]),
            TagsOrText::Text("text".to_string()),
        ])
    )
}

#[test]
fn test_empty() {
    let sections = rich("");

    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].value, "");
}

// #[test]
// fn test_sections() {
//     use bevy::color::palettes;

//     let default = TextStyle::default();

//     let red = TextStyle {
//         color: palettes::css::RED.into(),
//         ..Default::default()
//     };
//     let blue = TextStyle {
//         color: palettes::css::BLUE.into(),
//         ..Default::default()
//     };

//     let style_library = StyleRegistry::default().with_styles([
//         ("red".to_string(), red.clone()),
//         ("blue".to_string(), blue.clone()),
//     ]);

//     let sections = rich("test1[red]test2[]test3[blue]test4", &style_library);

//     assert_eq!(sections.len(), 4);

//     assert_eq!(sections[0].value, "test1");
//     assert_eq!(sections[0].style.color.0, default.color.0);
//     assert_eq!(sections[1].value, "test2");
//     assert_eq!(sections[1].style.color.0, red.color.0);
//     assert_eq!(sections[2].value, "test3");
//     assert_eq!(sections[2].style.color.0, default.color.0);
//     assert_eq!(sections[3].value, "test4");
//     assert_eq!(sections[3].style.color.0, blue.color.0);
// }
