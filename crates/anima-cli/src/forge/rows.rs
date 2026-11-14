//! A page of a K1 connection as JSON, so one pager and one renderer serve a connection that
//! two documents select differently (with and without checks, with and without jobs).

use serde::Serialize;
use serde_json::Value;

use crate::paging::Page;

#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct Rows(Value);

impl Rows {
    /// The page `connection` holds; a generated K1 answer always serialises.
    #[must_use]
    pub fn new(connection: &impl Serialize) -> Self {
        Self(serde_json::to_value(connection).unwrap_or(Value::Null))
    }

    fn edges(&self) -> &[Value] {
        self.0
            .get("edges")
            .and_then(Value::as_array)
            .map_or(&[], Vec::as_slice)
    }
}

impl Page for Rows {
    type Cursor = String;

    fn next_cursor(&self) -> Option<String> {
        let page = &self.0["pageInfo"];
        let more = page["hasNextPage"] == Value::Bool(true);
        page["endCursor"]
            .as_str()
            .filter(|_| more)
            .map(str::to_owned)
    }

    fn append(&mut self, later: Self) {
        if let (Some(edges), Value::Object(mut later)) = (
            self.0.get_mut("edges").and_then(Value::as_array_mut),
            later.0,
        ) {
            if let Some(Value::Array(more)) = later.remove("edges") {
                edges.extend(more);
            }
            if let Some(page) = later.remove("pageInfo") {
                self.0["pageInfo"] = page;
            }
        }
    }

    fn end(&mut self) {
        self.0["pageInfo"]["hasNextPage"] = Value::Bool(false);
    }

    fn edge_count(&self) -> usize {
        self.edges().len()
    }
}
