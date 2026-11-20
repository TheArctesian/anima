//! The reader of YAML for the syntax check: a tree of the one document, each node with its
//! line and column. It takes the events of the parser one at a time, so that a hostile file
//! stops it at a fixed depth and a fixed count. An alias is one node: nothing is expanded.

use yaml_rust2::{
    Event,
    parser::Parser,
    scanner::{Marker, TScalarStyle},
};

/// The deepest nesting of a file.
pub(super) const DEEPEST: usize = 64;
/// The most events of the parser for one file.
pub(super) const MOST_EVENTS: usize = 100_000;

/// A position in the text: both numbers start at 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Place {
    pub line: u32,
    pub column: u32,
}

impl Place {
    pub(super) const START: Self = Self { line: 1, column: 1 };

    fn of(mark: &Marker) -> Self {
        let number = |value: usize| u32::try_from(value).unwrap_or(u32::MAX);
        Self {
            line: number(mark.line()).max(1),
            column: number(mark.col()).saturating_add(1),
        }
    }
}

/// What a node is. A plain scalar that YAML reads as "no value" is `Null`.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Shape {
    Null,
    Text(String),
    List(Vec<Node>),
    /// Each key with its value, in the order of the file. A key can be any node.
    Map(Vec<(Node, Node)>),
    /// A reference to an anchor. The reader does not follow it.
    Alias,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Node {
    pub place: Place,
    pub shape: Shape,
}

/// Why a text gave no tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Unread {
    NotYaml(Place),
    TooDeep(Place),
    TooManyNodes(Place),
    MoreThanOneDocument(Place),
}

/// What is open while the events come: a list, or a map with the key that waits for a value.
enum Open {
    List(Vec<Node>),
    Map(Vec<(Node, Node)>, Option<Node>),
}

#[derive(Default)]
struct Tree {
    open: Vec<(Place, Open)>,
    root: Option<Node>,
}

impl Tree {
    fn put(&mut self, node: Node) {
        match self.open.last_mut() {
            None => self.root = Some(node),
            Some((_, Open::List(items))) => items.push(node),
            Some((_, Open::Map(entries, key))) => match key.take() {
                Some(key) => entries.push((key, node)),
                None => *key = Some(node),
            },
        }
    }

    fn open(&mut self, place: Place, open: Open) -> Result<(), Unread> {
        if self.open.len() >= DEEPEST {
            return Err(Unread::TooDeep(place));
        }
        self.open.push((place, open));
        Ok(())
    }

    fn close(&mut self) {
        if let Some((place, open)) = self.open.pop() {
            let shape = match open {
                Open::List(items) => Shape::List(items),
                Open::Map(entries, _) => Shape::Map(entries),
            };
            self.put(Node { place, shape });
        }
    }
}

fn scalar(text: String, style: TScalarStyle) -> Shape {
    let plain = style == TScalarStyle::Plain;
    if plain && matches!(text.as_str(), "" | "~" | "null" | "Null" | "NULL") {
        Shape::Null
    } else {
        Shape::Text(text)
    }
}

/// The one document of `text` as a tree; `None` for a text with no document.
///
/// # Errors
/// [`Unread`] for text that is not YAML, a second document, and a file that is deeper or
/// longer than the limits.
pub(super) fn read(text: &str) -> Result<Option<Node>, Unread> {
    let mut parser = Parser::new_from_str(text);
    let mut tree = Tree::default();
    let mut documents = 0_u8;
    let mut last = Place::START;
    for _ in 0..MOST_EVENTS {
        let (event, mark) = parser
            .next_token()
            .map_err(|error| Unread::NotYaml(Place::of(error.marker())))?;
        let place = Place::of(&mark);
        last = place;
        match event {
            Event::StreamEnd => return Ok(tree.root),
            Event::DocumentStart if documents > 0 => {
                return Err(Unread::MoreThanOneDocument(place));
            }
            Event::DocumentStart => documents += 1,
            Event::Alias(_) => tree.put(Node {
                place,
                shape: Shape::Alias,
            }),
            Event::Scalar(text, style, ..) => tree.put(Node {
                place,
                shape: scalar(text, style),
            }),
            Event::SequenceStart(..) => tree.open(place, Open::List(Vec::new()))?,
            Event::MappingStart(..) => tree.open(place, Open::Map(Vec::new(), None))?,
            Event::SequenceEnd | Event::MappingEnd => tree.close(),
            Event::Nothing | Event::StreamStart | Event::DocumentEnd => {}
        }
    }
    Err(Unread::TooManyNodes(last))
}
