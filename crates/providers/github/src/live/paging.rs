//! Paging: `per_page` and `page`, and the next page number from the `Link` header.

pub(super) fn query(page: &str) -> [(&'static str, &str); 2] {
    [("per_page", "100"), ("page", page)]
}

/// The page number of the `rel="next"` link in a `Link` header, or `None` on the last page.
/// Only the number is taken: the hub never follows a URL from the header, so a header that
/// names another host or path changes nothing.
#[must_use]
pub fn next_page(link_header: &str) -> Option<u32> {
    link_header.split(',').find_map(|link| {
        let (target, parameters) = link.split_once(';')?;
        let is_next = parameters
            .split(';')
            .any(|parameter| parameter.replace(' ', "") == "rel=\"next\"");
        if !is_next {
            return None;
        }
        let url = target.trim().strip_prefix('<')?.strip_suffix('>')?;
        let (_, query) = url.split_once('?')?;
        query
            .split('&')
            .find_map(|pair| pair.strip_prefix("page="))
            .and_then(|number| number.parse::<u32>().ok())
            .filter(|number| *number >= 1)
    })
}
