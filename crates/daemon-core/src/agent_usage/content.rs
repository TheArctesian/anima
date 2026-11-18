//! The `content` of a Claude Code message, read for two facts only: whether the operator typed
//! it, and how many tool calls it holds. The text is looked at where it lies in the line and
//! never copied or kept (AIS-018).

use std::fmt;

use serde::Deserialize;
use serde::de::{Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};

/// What a message's `content` holds.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Content {
    /// Its text is something the operator typed, not text the program wrote in their place.
    pub typed: bool,
    /// Its `tool_use` and `server_tool_use` blocks.
    pub tool_calls: u64,
}

/// Whether text was typed by the operator: text that does not start with a tag, or a slash
/// command or a shell line they typed. Text the program adds in the user's turn (reminders,
/// notifications, command output) starts with a tag of its own.
fn typed(text: &str) -> bool {
    const TYPED_TAGS: [&str; 3] = ["<command-name>", "<command-message>", "<bash-input>"];
    let text = text.trim_start();
    !text.is_empty()
        && (!text.starts_with('<') || TYPED_TAGS.iter().any(|tag| text.starts_with(tag)))
}

impl<'de> Deserialize<'de> for Content {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        input.deserialize_any(ContentVisitor)
    }
}

struct ContentVisitor;

impl<'de> Visitor<'de> for ContentVisitor {
    type Value = Content;

    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a text or a list of blocks")
    }

    fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<Content, E> {
        Ok(Content {
            typed: typed(text),
            tool_calls: 0,
        })
    }

    /// A list of blocks: typed when its first text block is typed and no block is a tool's
    /// result (a tool's result comes back in the user's turn).
    fn visit_seq<A: SeqAccess<'de>>(self, mut blocks: A) -> Result<Content, A::Error> {
        let mut first_text: Option<bool> = None;
        let mut result = false;
        let mut tool_calls: u64 = 0;
        while let Some(block) = blocks.next_element::<Block>()? {
            match block.kind {
                Kind::Text(typed) => {
                    first_text.get_or_insert(typed);
                }
                Kind::ToolCall => tool_calls = tool_calls.saturating_add(1),
                Kind::ToolResult => result = true,
                Kind::Other => {}
            }
        }
        Ok(Content {
            typed: first_text == Some(true) && !result,
            tool_calls,
        })
    }
}

/// One block of a list, by its `type` and, for a text block, whether its text is typed.
struct Block {
    kind: Kind,
}

enum Kind {
    Text(bool),
    ToolCall,
    ToolResult,
    Other,
}

impl<'de> Deserialize<'de> for Block {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        input.deserialize_map(BlockVisitor)
    }
}

struct BlockVisitor;

impl<'de> Visitor<'de> for BlockVisitor {
    type Value = Block;

    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a content block")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut fields: A) -> Result<Block, A::Error> {
        let mut kind: Option<BlockType> = None;
        let mut text: Option<Typed> = None;
        while let Some(key) = fields.next_key::<Key>()? {
            match key {
                Key::Type => kind = Some(fields.next_value()?),
                Key::Text => text = Some(fields.next_value()?),
                Key::Other => {
                    fields.next_value::<IgnoredAny>()?;
                }
            }
        }
        let kind = match kind {
            Some(BlockType::Text) => Kind::Text(text.is_some_and(|text| text.0)),
            Some(BlockType::ToolCall) => Kind::ToolCall,
            Some(BlockType::ToolResult) => Kind::ToolResult,
            Some(BlockType::Other) | None => Kind::Other,
        };
        Ok(Block { kind })
    }
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum Key {
    Type,
    Text,
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum BlockType {
    Text,
    #[serde(alias = "server_tool_use", rename = "tool_use")]
    ToolCall,
    ToolResult,
    #[serde(other)]
    Other,
}

/// Whether a text is typed by [`typed`], read where it lies and not kept.
struct Typed(bool);

impl<'de> Deserialize<'de> for Typed {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        input.deserialize_str(TypedVisitor)
    }
}

struct TypedVisitor;

impl Visitor<'_> for TypedVisitor {
    type Value = Typed;

    fn expecting(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str("a text")
    }

    fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<Typed, E> {
        Ok(Typed(typed(text)))
    }
}
