//! Aligned columns for a person (CMD-020). Cells are cleaned of control characters; colour is
//! added after the padding, so escape codes never shift a column (CMD-027).

use super::text::clean;

/// How a cell is coloured when colour is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Plain,
    Good,
    Bad,
    Warning,
    Muted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    text: String,
    tone: Tone,
}

/// Rows under a header (a list), or label and value pairs with no header (one record).
#[derive(Debug, Default)]
pub struct Table {
    header: Vec<&'static str>,
    rows: Vec<Vec<Cell>>,
}

const GAP: &str = "  ";

impl Cell {
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self::toned(text, Tone::Plain)
    }

    #[must_use]
    pub fn toned(text: &str, tone: Tone) -> Self {
        Self {
            text: clean(text),
            tone,
        }
    }

    /// A K1 `Status` (`UP`, `DOWN`, ...) in its colour.
    #[must_use]
    pub fn status(name: &str) -> Self {
        let tone = match name {
            "UP" => Tone::Good,
            "DOWN" => Tone::Bad,
            "DEGRADED" => Tone::Warning,
            _ => Tone::Muted,
        };
        Self::toned(name, tone)
    }

    fn width(&self) -> usize {
        self.text.chars().count()
    }
}

impl From<&str> for Cell {
    fn from(text: &str) -> Self {
        Self::new(text)
    }
}

impl From<String> for Cell {
    fn from(text: String) -> Self {
        Self::new(&text)
    }
}

impl Table {
    /// A list with these column names.
    #[must_use]
    pub fn new(header: Vec<&'static str>) -> Self {
        Self {
            header,
            rows: Vec::new(),
        }
    }

    /// One record: each row is a label and its value.
    #[must_use]
    pub fn record() -> Self {
        Self::default()
    }

    pub fn row(&mut self, cells: Vec<Cell>) {
        self.rows.push(cells);
    }

    /// A row of a record: `label` then `value`.
    pub fn pair(&mut self, label: &str, value: impl Into<Cell>) {
        self.rows
            .push(vec![Cell::toned(label, Tone::Muted), value.into()]);
    }

    /// The table as lines, each ending in a line break.
    #[must_use]
    pub fn render(&self, colour: bool) -> String {
        let header: Vec<Cell> = self.header.iter().map(|name| Cell::new(name)).collect();
        let lines = std::iter::once(&header)
            .filter(|header| !header.is_empty())
            .chain(&self.rows);
        let widths = self.widths(&header);
        let mut out = String::new();
        for cells in lines {
            let last = cells.len().saturating_sub(1);
            for (index, cell) in cells.iter().enumerate() {
                let pad = if index == last {
                    0
                } else {
                    widths[index] - cell.width()
                };
                out.push_str(&paint(&cell.text, cell.tone, colour));
                out.push_str(&" ".repeat(pad));
                out.push_str(if index == last { "\n" } else { GAP });
            }
        }
        out
    }

    fn widths(&self, header: &[Cell]) -> Vec<usize> {
        let mut widths = vec![0; header.len()];
        for cells in std::iter::once(header).chain(self.rows.iter().map(Vec::as_slice)) {
            if widths.len() < cells.len() {
                widths.resize(cells.len(), 0);
            }
            for (width, cell) in widths.iter_mut().zip(cells) {
                *width = (*width).max(cell.width());
            }
        }
        widths
    }
}

fn paint(text: &str, tone: Tone, colour: bool) -> String {
    let code = match tone {
        Tone::Plain => return text.to_owned(),
        Tone::Good => "32",
        Tone::Bad => "31",
        Tone::Warning => "33",
        Tone::Muted => "2",
    };
    if colour {
        format!("\u{1b}[{code}m{text}\u{1b}[0m")
    } else {
        text.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Table {
        let mut table = Table::new(vec!["NAME", "STATUS"]);
        table.row(vec!["nas".into(), Cell::toned("DOWN", Tone::Bad)]);
        table.row(vec!["laptop-é".into(), Cell::toned("UP", Tone::Good)]);
        table
    }

    #[test]
    fn cmd_020_columns_line_up_by_characters() {
        assert_eq!(
            sample().render(false),
            "NAME      STATUS\nnas       DOWN\nlaptop-é  UP\n"
        );
    }

    #[test]
    fn cmd_027_colour_does_not_shift_the_columns() {
        let painted = sample().render(true);
        assert!(
            painted.contains("nas       \u{1b}[31mDOWN\u{1b}[0m\n"),
            "{painted:?}"
        );
    }

    #[test]
    fn cmd_020_a_record_is_labels_and_values() {
        let mut table = Table::record();
        table.pair("Name", "nas");
        table.pair("Host", "10.0.0.2\u{1b}[2J");
        assert_eq!(table.render(false), "Name  nas\nHost  10.0.0.2 [2J\n");
    }
}
