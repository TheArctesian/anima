//! Linux (WebKitGTK): NOT COMPILED OR RUN by the builder (macOS only); the operator runs it.
//! WebKitGTK fails an untrusted load with `load-failed-with-tls-errors` and hands over the
//! `GTlsCertificate`. A pinned leaf is allowed for the host with
//! `WebContext::allow_tls_certificate_for_host` and the load is retried. Limits: the
//! allowance is per host (not port) and per web context, and the signal never fires for a
//! system-trusted certificate, so "a pin wins over system trust" (D-4) is not enforced here.

use tauri::Url;
use tauri::webview::PlatformWebview;
use webkit2gtk::glib::{ByteArray, prelude::ObjectExt};
use webkit2gtk::{WebContextExt, WebViewExt};

use super::TrustHook;
use crate::pin::{Fingerprint, Presented, Verdict};

/// Connects the TLS-error signal. Runs on the GTK main thread (`with_webview`).
pub fn install(platform: &PlatformWebview, hook: TrustHook) -> Result<String, String> {
    let view = platform.inner();
    let label = hook.label().to_owned();
    view.connect_load_failed_with_tls_errors(move |view, uri, certificate, _errors| {
        let Some((host, port)) = origin(uri) else {
            return true;
        };
        let der: ByteArray = certificate.property("certificate");
        // The signal fires only for certificates the system did not trust.
        let presented = Presented {
            host: host.clone(),
            port,
            sha256: Fingerprint::of(&der),
            system_trusted: false,
        };
        if hook.judge(&presented) == Verdict::Accept {
            if let Some(context) = view.context() {
                context.allow_tls_certificate_for_host(certificate, &host);
                view.load_uri(uri);
            }
        }
        // Handled: WebKit shows no error page of its own.
        true
    });
    Ok(format!("{label}: load-failed-with-tls-errors connected"))
}

fn origin(uri: &str) -> Option<(String, u16)> {
    let url = Url::parse(uri).ok()?;
    Some((url.host_str()?.to_owned(), url.port_or_known_default()?))
}
