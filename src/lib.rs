use bevy::{
    app::{Plugin, Update},
    core::Name,
    ecs::{component::Component, entity::Entity, query::Changed, system::Resource, world::World},
    hierarchy::DespawnRecursiveExt,
    prelude::{BuildChildren, Bundle, Deref, DerefMut, EntityCommands},
    text::{TextColor, TextFont, TextSpan},
    utils::HashMap,
};
use chumsky::{
    error::Cheap,
    primitive::{choice, just, none_of},
    Parser,
};

// What if our style registry was a HashMap<String, Entity>
// and we cloned every Component on Entity onto the TextSpan?

// What if our style registry was a HashMap<String, FnMut<&EntityCommands>> or whatever?
pub mod prelude {
    pub use crate::style_fn;
    pub use crate::RichText;
    pub use crate::RichTextPlugin;
    pub use crate::StyleRegistry;
}

#[derive(Default)]
pub struct TextStyle {
    font: TextFont,
    color: TextColor,
}

pub struct TextSection {
    value: String,
    tag: String,
}

#[derive(Component)]
pub struct RichText(pub String);

pub struct RichTextPlugin;
impl Plugin for RichTextPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(Update, update);
    }
}

fn update(world: &mut World) {
    let mut ents_query = world.query_filtered::<Entity, Changed<RichText>>();
    let mut rt_query = world.query::<&RichText>();

    let Some(mut registry) = world.remove_resource::<StyleRegistry>() else {
        return;
    };

    let ents = ents_query.iter(world).collect::<Vec<_>>();

    for ent in ents {
        bevy::log::info!("!!");
        world.commands().entity(ent).despawn_descendants();

        let Ok(rt) = rt_query.get(world, ent) else {
            continue;
        };

        let parsed = rich(&rt.0);

        let mut children = vec![];
        let mut cmds = world.commands();
        for section in parsed {
            let mut span_ent = cmds.spawn(TextSpan::new(section.value));

            let style = registry.get_mut_or_default(&section.tag);

            style(&mut span_ent);

            children.push(span_ent.id());
        }

        world.flush();

        for child in children {
            world.entity_mut(ent).add_child(child);
        }
    }
}

// type StyleFn = Box<dyn Fn() -> Box<dyn Bundle> + Send + Sync + 'static>;

// fn style<F, B>(f: F) -> StyleFn
// where
//     F: Fn() -> B + Send + Sync + 'static,
//     B: Bundle + 'static,
// {
//     Box::new(move || Box::new(f()))
// }

// type StyleFn = Box<dyn FnMut(&mut EntityCommands) + Send + Sync + 'static>;

// fn style_fn<F>(f: F) -> StyleFn
// where
//     F: FnMut(&mut EntityCommands) + Send + Sync + 'static,
// {
//     Box::new(f)
// }

// fn style_fn<F>(f: F) -> StyleFn
// where
//     F: Bundle,
// {
//     Box::new(|e: &mut EntityCommands| {
//         e.insert(f);
//     })
// }

type StyleFn = Box<dyn FnMut(&mut EntityCommands) + Send + Sync + 'static>;

pub fn style_fn<F, C>(f: C) -> StyleFn
where
    F: Bundle + Send + Sync + 'static,
    C: Fn() -> F + Send + Sync + 'static,
{
    Box::new(move |e: &mut EntityCommands| {
        e.insert(f());
    })
}

#[derive(Resource, Deref, DerefMut)]
//pub struct StyleRegistry(pub HashMap<String, Entity>);
pub struct StyleRegistry(HashMap<String, StyleFn>);
impl<'a> StyleRegistry {
    pub fn get_mut_or_default(&mut self, tag: &str) -> &mut StyleFn {
        if self.0.contains_key(tag) {
            return self.0.get_mut(tag).unwrap();
        }

        return self.0.get_mut("").unwrap();
    }

    pub fn get_default(&self) -> &StyleFn {
        &self.0[""]
    }
    pub fn get_or_default(&self, tag: &str) -> &StyleFn {
        self.0.get(tag).unwrap_or_else(|| self.get_default())
    }
    pub fn with_styles<T>(mut self, styles: T) -> Self
    where
        T: IntoIterator<Item = (String, StyleFn)>,
    {
        self.0.extend(styles);
        self
    }
}
impl Default for StyleRegistry {
    fn default() -> Self {
        Self(HashMap::from([(
            "".to_string(),
            style_fn(|| Name::new("test")),
        )]))
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
