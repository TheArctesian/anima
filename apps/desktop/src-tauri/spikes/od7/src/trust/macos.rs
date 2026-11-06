//! macOS: a navigation-delegate proxy. wry's `WryNavigationDelegate` does not implement
//! `webView:didReceiveAuthenticationChallenge:completionHandler:`, so WebKit applies system
//! trust and an untrusted certificate just fails. The proxy implements that one method and
//! forwards every other selector to wry's delegate.

use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr::null_mut;

use block2::DynBlock;
use core_foundation::base::TCFType;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, ProtocolObject, Sel};
use objc2::{ClassType, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_foundation::{
    NSObjectProtocol, NSURLAuthenticationChallenge, NSURLAuthenticationMethodServerTrust,
    NSURLCredential, NSURLSessionAuthChallengeDisposition as Disposition,
};
use objc2_web_kit::{WKNavigationDelegate, WKWebView};
use security_framework::trust::SecTrust;
use tauri::webview::PlatformWebview;

use super::TrustHook;
use crate::pin::{Fingerprint, Presented, Verdict};

type Completion = DynBlock<dyn Fn(Disposition, *mut NSURLCredential)>;

pub struct ProxyIvars {
    inner: Retained<ProtocolObject<dyn WKNavigationDelegate>>,
    hook: TrustHook,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "AnimaOd7PinningDelegate"]
    #[ivars = ProxyIvars]
    struct PinningDelegate;

    impl PinningDelegate {
        /// WebKit asks this once, when the delegate is set, for each optional method.
        #[unsafe(method(respondsToSelector:))]
        fn responds_to(&self, selector: Sel) -> bool {
            let own: bool = unsafe { msg_send![super(self), respondsToSelector: selector] };
            own || self.ivars().inner.respondsToSelector(selector)
        }

        #[unsafe(method(forwardingTargetForSelector:))]
        fn forward_to(&self, selector: Sel) -> *mut AnyObject {
            if self.ivars().inner.respondsToSelector(selector) {
                Retained::as_ptr(&self.ivars().inner).cast::<AnyObject>().cast_mut()
            } else {
                null_mut()
            }
        }
    }

    unsafe impl NSObjectProtocol for PinningDelegate {}

    unsafe impl WKNavigationDelegate for PinningDelegate {
        #[unsafe(method(webView:didReceiveAuthenticationChallenge:completionHandler:))]
        fn challenge(
            &self,
            _view: &WKWebView,
            challenge: &NSURLAuthenticationChallenge,
            done: &Completion,
        ) {
            answer(&self.ivars().hook, challenge, done);
        }
    }
);

thread_local! {
    /// `navigationDelegate` is weak; the proxies live here, on the main thread, until exit.
    static PROXIES: RefCell<Vec<Retained<PinningDelegate>>> = const { RefCell::new(Vec::new()) };
}

/// Puts the proxy in front of wry's delegate. Runs on the main thread (`with_webview`).
pub fn install(platform: &PlatformWebview, hook: TrustHook) -> Result<String, String> {
    let mtm = MainThreadMarker::new().ok_or("not on the main thread")?;
    // SAFETY: `inner()` is the live WKWebView (a wry subclass) owned by this web view.
    let view = unsafe { platform.inner().cast::<WKWebView>().as_ref() }.ok_or("no WKWebView")?;
    // SAFETY: plain property read on the main thread.
    let inner = unsafe { view.navigationDelegate() }.ok_or("no navigation delegate")?;
    let label = hook.label().to_owned();
    let proxy = mtm
        .alloc::<PinningDelegate>()
        .set_ivars(ProxyIvars { inner, hook });
    // SAFETY: NSObject's designated initialiser.
    let proxy: Retained<PinningDelegate> = unsafe { msg_send![super(proxy), init] };
    // SAFETY: the proxy is retained in PROXIES for the life of the process.
    unsafe { view.setNavigationDelegate(Some(ProtocolObject::from_ref(&*proxy))) };
    // SAFETY: plain property read on the main thread.
    let installed = unsafe { view.navigationDelegate() }
        .is_some_and(|d| std::ptr::addr_eq(Retained::as_ptr(&d), Retained::as_ptr(&proxy)));
    PROXIES.with_borrow_mut(|proxies| proxies.push(proxy));
    if installed {
        Ok(format!("{label}: proxy is the navigation delegate"))
    } else {
        Err(format!("{label}: setNavigationDelegate did not take"))
    }
}

/// Server trust only; any other challenge (HTTP auth, client certificates) gets the default.
fn answer(hook: &TrustHook, challenge: &NSURLAuthenticationChallenge, done: &Completion) {
    let space = challenge.protectionSpace();
    // SAFETY: an immutable Foundation constant.
    if !space
        .authenticationMethod()
        .isEqualToString(unsafe { NSURLAuthenticationMethodServerTrust })
    {
        done.call((Disposition::PerformDefaultHandling, null_mut()));
        return;
    }
    // SAFETY: `serverTrust` returns a SecTrustRef owned by the protection space (get rule).
    let trust_ref: *mut c_void = unsafe { msg_send![&*space, serverTrust] };
    if trust_ref.is_null() {
        done.call((Disposition::CancelAuthenticationChallenge, null_mut()));
        return;
    }
    // SAFETY: a live, non-null SecTrustRef; the get rule retains it for the wrapper's life.
    let trust = unsafe { SecTrust::wrap_under_get_rule(trust_ref.cast()) };
    let system_trusted = trust.evaluate_with_error().is_ok();
    let Some(leaf) = trust.chain().into_iter().next() else {
        done.call((Disposition::CancelAuthenticationChallenge, null_mut()));
        return;
    };
    let sha256 = Fingerprint::of(&leaf.to_der());
    let host = space.host().to_string();
    // An out-of-range port matches no pin, so it gets the system's handling.
    let port = u16::try_from(space.port()).unwrap_or(0);
    let presented = Presented {
        host,
        port,
        sha256,
        system_trusted,
    };
    match hook.judge(&presented) {
        Verdict::Accept => {
            // SAFETY: `credentialForTrust:` takes the same SecTrustRef; the result is autoreleased.
            let credential: Retained<NSURLCredential> =
                unsafe { msg_send![NSURLCredential::class(), credentialForTrust: trust_ref] };
            done.call((
                Disposition::UseCredential,
                Retained::as_ptr(&credential).cast_mut(),
            ));
        }
        Verdict::Default => done.call((Disposition::PerformDefaultHandling, null_mut())),
        Verdict::Untrusted(_) | Verdict::Mismatch { .. } => {
            done.call((Disposition::CancelAuthenticationChallenge, null_mut()));
        }
    }
}
