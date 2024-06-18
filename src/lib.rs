use bevy::{
    ecs::system::Resource,
    prelude::{Deref, DerefMut},
    text::{TextSection, TextStyle},
    utils::HashMap,
};
use chumsky::{
    error::Cheap,
    primitive::{choice, just, none_of},
    Parser,
};

pub mod prelude {
    pub use crate::rich;
    pub use crate::StyleRegistry;
}

#[derive(Resource, Deref, DerefMut)]
pub struct StyleRegistry(pub HashMap<String, TextStyle>);
impl StyleRegistry {
    pub fn get_default(&self) -> &TextStyle {
        &self.0[""]
    }
    pub fn get_or_default(&self, tag: &str) -> &TextStyle {
        self.0.get(tag).unwrap_or_else(|| self.get_default())
    }
    pub fn with_styles<T>(mut self, styles: T) -> Self
    where
        T: IntoIterator<Item = (String, TextStyle)>,
    {
        self.0.extend(styles);
        self
    }
}
impl Default for StyleRegistry {
    fn default() -> Self {
        Self(HashMap::from([("".to_string(), TextStyle::default())]))
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

pub fn rich(text: &str, styles: &StyleRegistry) -> Vec<TextSection> {
    let mut sections = vec![];
    let mut style = styles.get_default();

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
                style: style.clone(),
            });

            return sections;
        }
    };

    for t in tags_or_text {
        match t {
            TagOrText::Text(value) => sections.push(TextSection {
                value,
                style: style.clone(),
            }),
            TagOrText::Tag(tag) => style = styles.get_or_default(&tag),
        }
    }

    if sections.is_empty() {
        sections.push(TextSection {
            value: "".to_string(),
            style: style.clone(),
        });
    }

    sections
}

#[test]
fn test_parser() {
    assert_eq!(
        tags_or_text().parse("[bold]"),
        Ok(vec![TagOrText::Tag("bold".to_string())])
    );
    assert_eq!(
        tags_or_text().parse("[[horse]]"),
        Ok(vec![TagOrText::Text("[horse]".to_string())])
    );
    assert_eq!(
        tags_or_text().parse("[bold]Bold Text[italic]Italic Text"),
        Ok(vec![
            TagOrText::Tag("bold".to_string()),
            TagOrText::Text("Bold Text".to_string()),
            TagOrText::Tag("italic".to_string()),
            TagOrText::Text("Italic Text".to_string()),
        ])
    );
    assert_eq!(
        tags_or_text().parse("[]Text[]"),
        Ok(vec![
            TagOrText::Tag("".to_string()),
            TagOrText::Text("Text".to_string()),
            TagOrText::Tag("".to_string()),
        ])
    );
    assert_eq!(
        tags_or_text().parse("[[]]][]"),
        Ok(vec![
            TagOrText::Text("[]]".to_string()),
            TagOrText::Tag("".to_string()),
        ])
    );
}

#[test]
fn test_empty() {
    let sections = rich("", &StyleRegistry::default());

    assert!(sections.len() == 1);
    assert!(sections[0].value == "");
}

#[test]
fn test_sections() {
    use bevy::render::color::Color;

    let red = TextStyle {
        color: Color::rgb(1., 0., 0.),
        ..Default::default()
    };
    let blue = TextStyle {
        color: Color::rgb(0., 0., 1.),
        ..Default::default()
    };

    let style_library = StyleRegistry::default()
        .with_styles([("red".to_string(), red), ("blue".to_string(), blue)]);

    let sections = rich("test1[red]test2[]test3[blue]test4", &style_library);

    assert_eq!(sections.len(), 4);

    assert_eq!(sections[0].value, "test1");
    assert_eq!(sections[0].style.color, Color::WHITE);
    assert_eq!(sections[1].value, "test2");
    assert_eq!(sections[1].style.color, Color::RED);
    assert_eq!(sections[2].value, "test3");
    assert_eq!(sections[2].style.color, Color::WHITE);
    assert_eq!(sections[3].value, "test4");
    assert_eq!(sections[3].style.color, Color::BLUE);
}
