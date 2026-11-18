const TOKENS: &str = include_str!("installed-tokens.css");
const STYLES: &str = include_str!("installed.css");
const DOORS: &str = include_str!("doors.svg");
const GITHUB: &str = include_str!("github.svg");

/// Renders the fixed title and message with the bundled marks (UR-7, D13).
pub(super) fn document(title: &str, message: &str) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
         <title>{title} | Anima.ing</title><style>{TOKENS}{STYLES}</style></head>\
         <body><main>{DOORS}<div class=\"result\">{GITHUB}<h1>{title}</h1><p>{message}</p>\
         </div></main></body></html>"
    )
}
