use chumsky::{
    error::Cheap,
    primitive::{choice, just, none_of},
    Parser,
};

#[derive(Default)]
pub(crate) struct TextSection {
    pub(crate) value: String,
    pub(crate) tags: Vec<String>,
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

pub fn parse_richtext(text: &str) -> Vec<TextSection> {
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
    let sections = parse_richtext("");

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
