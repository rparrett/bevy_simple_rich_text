# bevy_simple_rich_text

[![crates.io](https://img.shields.io/crates/v/bevy_simple_rich_text.svg)](https://crates.io/crates/bevy_simple_rich_text)
[![docs](https://docs.rs/bevy_simple_rich_text/badge.svg)](https://docs.rs/bevy_simple_rich_text)
[![Following released Bevy versions](https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue)](https://bevyengine.org/learn/book/plugin-development/#main-branch-tracking)

A tiny, unambitious rich text helper for `bevy_ui` with a simple bbcode-inspired syntax.

## Usage

```rust
// Register style tags by spawning `StyleTag` with `TextFont`, `TextColor`,
// and any other arbitrary Component.
commands.spawn((
    StyleTag::new("lg"),
    TextFont {
        font_size: 40.,
        ..default()
    },
));
commands.spawn((
    StyleTag::new("fancy"),
    TextColor(Color::hsl(0., 0.9, 0.7)),
    FancyText,
));

// And use them
commands.spawn(RichText::new("[lg]Hello [lg,fancy]World"));
```

See also [`examples/advanced.rs`](./examples/advanced.rs).

## Performance

Modifying a `RichText` completely rebuilds the `TextSpans`, so it's probably pretty slow.

But you can attach arbitrary marker components to styles to achieve fast animations.

## Compatibility

| `bevy_simple_rich_text`  | `bevy` |
| :--                      | :--    |
| `0.1`                    | `0.14` |

## Contributing

Please feel free to open a PR. The goal of this project is for it to stay simple and maintainable.

Please keep PRs small and scoped to a single feature or fix.
