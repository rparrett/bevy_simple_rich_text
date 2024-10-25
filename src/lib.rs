use bevy::{
    app::{Plugin, Update},
    asset::Assets,
    ecs::{component::Component, entity::Entity, query::Changed, system::Resource, world::World},
    hierarchy::DespawnRecursiveExt,
    prelude::{BuildChildren, Deref, DerefMut, FromWorld, Text},
    scene::{DynamicScene, DynamicSceneBuilder, SceneSpawner},
    text::TextSpan,
    utils::HashMap,
};
use chumsky::{
    error::Cheap,
    primitive::{choice, just, none_of},
    Parser,
};

// TODO consider not making users mess around with the hashmap.
// just let them spawn stuff with a RegisteredStyle component or something.
// We would have to filter these components when cloning.

pub mod prelude {
    pub use crate::RichText;
    pub use crate::RichTextPlugin;
    pub use crate::StyleRegistry;
}

#[derive(Default)]

pub struct TextSection {
    value: String,
    tag: String,
}

#[derive(Component)]
#[require(Text)]
pub struct RichText(pub String);

pub struct RichTextPlugin;
impl Plugin for RichTextPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<StyleRegistry>();
        app.add_systems(Update, update);
    }
}

fn update(world: &mut World) {
    let mut ents_query = world.query_filtered::<Entity, Changed<RichText>>();
    let mut rt_query = world.query::<&RichText>();

    let Some(registry) = world.remove_resource::<StyleRegistry>() else {
        return;
    };

    let ents = ents_query.iter(world).collect::<Vec<_>>();

    for ent in ents {
        world.commands().entity(ent).despawn_descendants();
        world.flush();

        let Ok(rt) = rt_query.get(world, ent) else {
            continue;
        };

        let parsed = rich(&rt.0);

        for section in parsed {
            let style_ent = registry.get_or_default(&section.tag);

            // Clone components from the style entity onto a new entity

            let mut scene_spawner = SceneSpawner::default();
            let scene = DynamicSceneBuilder::from_world(world)
                .extract_entity(*style_ent)
                .build();

            let scene_id = world.resource_mut::<Assets<DynamicScene>>().add(scene);
            let instance_id = scene_spawner.spawn_dynamic_sync(world, &scene_id).unwrap();

            let span_ent = scene_spawner
                .iter_instance_entities(instance_id)
                .next()
                .unwrap();

            // Make that new entity a `TextSpan` and add it as as a child
            // to our `RichText`.

            world
                .entity_mut(span_ent)
                .insert(TextSpan::new(section.value));

            world.entity_mut(ent).add_child(span_ent);
        }
    }

    world.insert_resource(registry);
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
    pub fn with_styles<T>(mut self, styles: T) -> Self
    where
        T: IntoIterator<Item = (String, Entity)>,
    {
        self.0.extend(styles);
        self
    }
}
impl FromWorld for StyleRegistry {
    fn from_world(world: &mut World) -> Self {
        Self(HashMap::from([("".to_string(), world.spawn_empty().id())]))
    }
}

#[derive(Debug, PartialEq, Eq)]
enum TagOrText {
    Tag(String),
    Text(String),
}

fn escaped_bracket() -> impl Parser<char, String, Error = Cheap<char>> {
    just('[')
        .ignore_then(just('['))
        .or(just(']').ignore_then(just(']')))
        .map(|c| c.to_string())
}

fn tag() -> impl Parser<char, TagOrText, Error = Cheap<char>> {
    not_end_bracket()
        .repeated()
        .delimited_by(just('['), just(']'))
        .collect::<String>()
        .map(TagOrText::Tag)
}

fn not_end_bracket() -> impl Parser<char, String, Error = Cheap<char>> {
    none_of("]").repeated().at_least(1).collect::<String>()
}

fn not_any_bracket() -> impl Parser<char, String, Error = Cheap<char>> {
    none_of("[]").repeated().at_least(1).collect::<String>()
}

fn stray_end_bracket() -> impl Parser<char, String, Error = Cheap<char>> {
    just(']').map(|c| c.to_string())
}

fn text() -> impl Parser<char, TagOrText, Error = Cheap<char>> {
    choice((escaped_bracket(), not_any_bracket(), stray_end_bracket()))
        .repeated()
        .at_least(1)
        .collect::<String>()
        .map(TagOrText::Text)
}

fn tags_or_text() -> impl Parser<char, Vec<TagOrText>, Error = Cheap<char>> {
    choice((text(), tag())).repeated().collect::<Vec<_>>()
}

pub fn rich(text: &str) -> Vec<TextSection> {
    let mut sections = vec![];
    let mut current_tag = "".to_string();

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
                tag: current_tag,
            });

            return sections;
        }
    };

    for t in tags_or_text {
        match t {
            TagOrText::Text(value) => sections.push(TextSection {
                value,
                tag: current_tag.clone(),
            }),
            TagOrText::Tag(tag) => current_tag = tag,
        }
    }

    if sections.is_empty() {
        sections.push(TextSection {
            value: "".to_string(),
            tag: "".to_string(),
        });
    }

    sections
}

// #[test]
// fn test_parser() {
//     assert_eq!(
//         tags_or_text().parse("[bold]"),
//         Ok(vec![TagOrText::Tag("bold".to_string())])
//     );
//     assert_eq!(
//         tags_or_text().parse("[[horse]]"),
//         Ok(vec![TagOrText::Text("[horse]".to_string())])
//     );
//     assert_eq!(
//         tags_or_text().parse("[bold]Bold Text[italic]Italic Text"),
//         Ok(vec![
//             TagOrText::Tag("bold".to_string()),
//             TagOrText::Text("Bold Text".to_string()),
//             TagOrText::Tag("italic".to_string()),
//             TagOrText::Text("Italic Text".to_string()),
//         ])
//     );
//     assert_eq!(
//         tags_or_text().parse("[]Text[]"),
//         Ok(vec![
//             TagOrText::Tag("".to_string()),
//             TagOrText::Text("Text".to_string()),
//             TagOrText::Tag("".to_string()),
//         ])
//     );
//     assert_eq!(
//         tags_or_text().parse("[[]]][]"),
//         Ok(vec![
//             TagOrText::Text("[]]".to_string()),
//             TagOrText::Tag("".to_string()),
//         ])
//     );
// }

// #[test]
// fn test_empty() {
//     let sections = rich("", &StyleRegistry::default());

//     assert_eq!(sections.len(), 1);
//     assert_eq!(sections[0].value, "");
// }

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
