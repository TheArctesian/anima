//! The pages the test servers serve to device web views. Pages report by `fetch` to the
//! server they came from, never by IPC.

/// Schemes a hostile page may try to navigate to (check 4). Each must be refused.
pub const NAVIGATION_TARGETS: &[&str] = &[
    "file:///etc/hosts",
    "data:text/html,od7",
    "tauri://localhost/index.html",
    "ipc://localhost/spike_probe",
    "anima-od7:probe",
];

/// A page and, for `/store/set`, the cookie the response sets.
pub struct Page {
    pub html: String,
    pub set_cookie: Option<String>,
}

const REPORT: &str = "const report = (name, body) => fetch('/report?name=' + name, \
    { method: 'POST', body: JSON.stringify(body) });\
    const query = new URLSearchParams(location.search);\
    const who = query.get('who');";

pub fn page(path: &str, query: &str) -> Option<Page> {
    let script = match path {
        "/blank" => String::new(),
        "/store/set" => STORE_SET.to_owned(),
        "/store/read" => STORE_READ.to_owned(),
        "/ipc" => IPC.to_owned(),
        "/nav" => navigation(),
        "/nav/landed" => "report('nav-landed', {});".to_owned(),
        "/tls" => "report('tls-loaded', { step: query.get('step') });".to_owned(),
        _ => return None,
    };
    let set_cookie = (path == "/store/set")
        .then(|| nonce(query))
        .flatten()
        .map(|n| format!("spike_http={n}; Path=/"));
    let html = format!(
        "<!doctype html><meta charset=\"utf-8\"><title>od7</title><script>{REPORT}{script}</script>"
    );
    Some(Page { html, set_cookie })
}

/// The `n` query value, only if it is 1 to 32 hex digits (it goes into a header).
fn nonce(query: &str) -> Option<&str> {
    query_value(query, "n")
        .filter(|n| (1..=32).contains(&n.len()) && n.bytes().all(|b| b.is_ascii_hexdigit()))
}

pub fn query_value<'q>(query: &'q str, key: &str) -> Option<&'q str> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find_map(|(k, v)| (k == key).then_some(v))
}

const STORE_SET: &str = "const n = query.get('n');\
    localStorage.setItem('spike', n);\
    document.cookie = 'spike_js=' + n + '; path=/';\
    report('store-set', { who, cookie: document.cookie, ls: localStorage.getItem('spike') });";

const STORE_READ: &str =
    "report('store-read', { who, cookie: document.cookie, ls: localStorage.getItem('spike') });";

/// Tries IPC three ways: Tauri's own invoke, a raw message to the IPC handler, and a fetch
/// to the IPC scheme. Results come back by `fetch`; Rust separately records any command run.
const IPC: &str = "(async () => {\
    const within = (p) => Promise.race([p, new Promise((r) => setTimeout(() => r('timeout'), 3000))]);\
    const out = { internals: typeof window.__TAURI_INTERNALS__ };\
    try {\
      const r = await within(window.__TAURI_INTERNALS__.invoke('spike_probe', { from: 'device-invoke' }));\
      out.invoke = r === 'timeout' ? 'timeout' : 'resolved';\
    } catch (e) { out.invoke = 'rejected: ' + String(e).slice(0, 160); }\
    try {\
      window.webkit.messageHandlers.ipc.postMessage(JSON.stringify({ cmd: 'spike_probe', callback: 1, error: 2, payload: { from: 'device-raw' }, options: {} }));\
      out.raw = 'posted';\
    } catch (e) { out.raw = 'threw: ' + String(e).slice(0, 80); }\
    try {\
      const r = await within(fetch('ipc://localhost/spike_probe', { method: 'POST', body: JSON.stringify({ from: 'device-fetch' }), headers: { 'Content-Type': 'application/json', 'Tauri-Callback': '1', 'Tauri-Error': '2' } }));\
      out.fetch = r === 'timeout' ? 'timeout' : 'status ' + r.status;\
    } catch (e) { out.fetch = 'rejected: ' + String(e).slice(0, 80); }\
    report('ipc', out);\
  })();";

/// Tries each target in turn; a refused navigation leaves the page alive to report.
fn navigation() -> String {
    let targets = serde_json::to_string(NAVIGATION_TARGETS).unwrap_or_else(|_| "[]".into());
    format!(
        "(async () => {{\
          const targets = {targets};\
          const sleep = (ms) => new Promise((r) => setTimeout(r, ms));\
          for (let i = 0; i < targets.length; i++) {{\
            try {{ location.href = targets[i]; }} catch (e) {{}}\
            await sleep(700);\
            await report('nav-alive', {{ i, href: location.href }});\
          }}\
          location.href = '/nav/landed';\
        }})();"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonce_is_hex_only() {
        assert_eq!(nonce("who=a&n=0aF9"), Some("0aF9"));
        assert_eq!(nonce("n=abc;%20Secure"), None);
        assert_eq!(nonce("n="), None);
        assert_eq!(nonce("n=0123456789abcdef0123456789abcdef0"), None);
    }

    #[test]
    fn set_cookie_only_for_store_set() {
        assert!(page("/store/read", "n=ab").is_some_and(|p| p.set_cookie.is_none()));
        assert_eq!(
            page("/store/set", "n=ab")
                .and_then(|p| p.set_cookie)
                .as_deref(),
            Some("spike_http=ab; Path=/")
        );
        assert!(page("/elsewhere", "").is_none());
    }
}
