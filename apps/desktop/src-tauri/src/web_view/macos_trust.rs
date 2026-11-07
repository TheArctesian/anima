//! macOS trust hook (WEB-008, OD-7, R-069): a navigation-delegate proxy in front of wry's.
//!
//! wry's `WryNavigationDelegate` has no `webView:didReceiveAuthenticationChallenge:
//! completionHandler:`, so without this `WebKit` applies system trust and a pin is never
//! asked. The proxy implements that one method and forwards every other selector to wry's
//! delegate, which keeps the `http`/`https` filter and the page-load events.
//!
//! This module only reads the challenge and carries the answer back. Which answer, and what
//! the operator is told, is decided in safe Rust by the `Decide` it is given.

#![expect(
    unsafe_code,
    reason = "R-069: WebKit's trust challenge is reachable only through Objective-C"
)]

use std::{ffi::c_void, ptr::null_mut};

use block2::DynBlock;
use core_foundation::base::TCFType;
use objc2::{
    ClassType, DefinedClass, MainThreadMarker, MainThreadOnly, define_class,
    ffi::{OBJC_ASSOCIATION_RETAIN_NONATOMIC, objc_setAssociatedObject},
    msg_send,
    rc::{Retained, Weak},
    runtime::{AnyObject, NSObject, ProtocolObject, Sel},
};
use objc2_foundation::{
    NSObjectProtocol, NSURLAuthenticationChallenge, NSURLAuthenticationMethodServerTrust,
    NSURLCredential, NSURLSessionAuthChallengeDisposition as Disposition,
};
use objc2_web_kit::{WKNavigationDelegate, WKWebView};
use security_framework::trust::SecTrust;
use tauri::webview::PlatformWebview;

use super::{
    Fingerprint, WebViewError,
    trust::{Answer, Presented},
};

/// Decides one challenge. The second argument evaluates system trust, when asked.
pub type Decide = Box<dyn Fn(&Presented<'_>, &dyn Fn() -> bool) -> Answer>;

type Completion = DynBlock<dyn Fn(Disposition, *mut NSURLCredential)>;

/// The key the proxy hangs on its web view by: this static's address, unique to this module.
static PROXY_KEY: u8 = 0;

pub struct ProxyIvars {
    /// wry's delegate, which gets every selector but the challenge. Weak: wry's own web view
    /// owns it, and it holds the `WKWebView` (its page-load handler), which holds this proxy;
    /// a strong reference here would make a cycle, and a closed view would never be freed.
    inner: Weak<ProtocolObject<dyn WKNavigationDelegate>>,
    decide: Decide,
}

define_class!(
    // SAFETY: NSObject has no subclassing rules this class breaks. It overrides two NSObject
    // methods with their own signatures: `respondsToSelector:` answers a `bool` (objc2's
    // `BOOL`), and `forwardingTargetForSelector:` answers an object at +0 (not retained for
    // the caller, kept alive by wry's web view) or nil, as NSObject's does. `Drop` is not
    // implemented and the ivars are plain Rust values.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "AnimaPinningDelegate"]
    #[ivars = ProxyIvars]
    struct PinningDelegate;

    impl PinningDelegate {
        /// `WebKit` asks this, when the delegate is set, for each optional method it may call.
        #[unsafe(method(respondsToSelector:))]
        fn responds_to(&self, selector: Sel) -> bool {
            Self::class().responds_to(selector)
                || self
                    .ivars()
                    .inner
                    .load()
                    .is_some_and(|inner| inner.respondsToSelector(selector))
        }

        #[unsafe(method(forwardingTargetForSelector:))]
        fn forward_to(&self, selector: Sel) -> *mut AnyObject {
            match self.ivars().inner.load() {
                // +0: wry's web view still owns the delegate after this `Retained` drops.
                Some(inner) if inner.respondsToSelector(selector) => {
                    Retained::as_ptr(&inner).cast::<AnyObject>().cast_mut()
                }
                _ => null_mut(),
            }
        }
    }

    // SAFETY: NSObjectProtocol has no requirements beyond being an NSObject.
    unsafe impl NSObjectProtocol for PinningDelegate {}

    // SAFETY: the one method below has the signature WebKit declares for it.
    unsafe impl WKNavigationDelegate for PinningDelegate {
        /// Calls `done` exactly once, on every path: `reply` returns, it never answers.
        #[unsafe(method(webView:didReceiveAuthenticationChallenge:completionHandler:))]
        fn challenge(
            &self,
            _view: &WKWebView,
            challenge: &NSURLAuthenticationChallenge,
            done: &Completion,
        ) {
            let (disposition, credential) = reply(&self.ivars().decide, challenge);
            let credential = credential
                .as_ref()
                .map_or(null_mut(), |credential| Retained::as_ptr(credential).cast_mut());
            done.call((disposition, credential));
        }
    }
);

/// Puts the proxy in front of wry's delegate on `platform`'s `WKWebView`. Runs on the main
/// thread (`Webview::with_webview`). On an error the view must load nothing.
///
/// The view owns the proxy (an associated object, retained), because `navigationDelegate` is
/// weak: the proxy lives exactly as long as the view. Popping out (`Webview::reparent`) moves
/// this same `WKWebView` to another window's content view, so the proxy and the pin go with it.
pub fn install(platform: &PlatformWebview, decide: Decide) -> Result<(), WebViewError> {
    let main_thread = MainThreadMarker::new().ok_or(WebViewError::Native)?;
    let view = web_view(platform)?;
    // SAFETY: a property read on the main thread.
    let inner = unsafe { view.navigationDelegate() }.ok_or(WebViewError::Native)?;
    let inner = Weak::from_retained(&inner);
    let proxy = main_thread
        .alloc::<PinningDelegate>()
        .set_ivars(ProxyIvars { inner, decide });
    // SAFETY: NSObject's designated initialiser, once, on a new allocation with its ivars set.
    let proxy: Retained<PinningDelegate> = unsafe { msg_send![super(proxy), init] };
    // SAFETY: both pointers are live objects; the key is this module's own static, and
    // RETAIN_NONATOMIC makes the view hold a strong reference, released with the view. Only
    // the main thread touches it.
    unsafe {
        objc_setAssociatedObject(
            std::ptr::from_ref(view).cast::<AnyObject>().cast_mut(),
            (&raw const PROXY_KEY).cast::<c_void>(),
            Retained::as_ptr(&proxy).cast::<AnyObject>().cast_mut(),
            OBJC_ASSOCIATION_RETAIN_NONATOMIC,
        );
    }
    // SAFETY: a property write on the main thread; the view retains the proxy (above), so the
    // weak property never dangles.
    unsafe { view.setNavigationDelegate(Some(ProtocolObject::from_ref(&*proxy))) };
    // SAFETY: a property read on the main thread.
    let current = unsafe { view.navigationDelegate() };
    let installed = current.is_some_and(|delegate| {
        std::ptr::addr_eq(Retained::as_ptr(&delegate), Retained::as_ptr(&proxy))
    });
    installed.then_some(()).ok_or(WebViewError::Native)
}

/// The `WKWebView` (a wry subclass) of a device web view, for as long as `platform` is
/// borrowed. Public for the pinning proof (`tests/pinning`), which checks with a weak
/// reference that a closed view is freed.
pub fn web_view(platform: &PlatformWebview) -> Result<&WKWebView, WebViewError> {
    // SAFETY: on macOS `inner()` is the live `WKWebView` of this web view, and wry keeps it
    // while `platform` exists; the reference cannot outlive the borrow of `platform`.
    unsafe { platform.inner().cast::<WKWebView>().as_ref() }.ok_or(WebViewError::Native)
}

/// Reads one challenge and turns the safe answer into `WebKit`'s. Server trust only: any
/// other challenge (HTTP auth, client certificates) gets the default, as without the hook.
fn reply(
    decide: &Decide,
    challenge: &NSURLAuthenticationChallenge,
) -> (Disposition, Option<Retained<NSURLCredential>>) {
    let space = challenge.protectionSpace();
    // SAFETY: reads an immutable Foundation constant.
    let server_trust = unsafe { NSURLAuthenticationMethodServerTrust };
    if !space.authenticationMethod().isEqualToString(server_trust) {
        return (Disposition::PerformDefaultHandling, None);
    }
    // SAFETY: `serverTrust` is a `SecTrustRef` property of the protection space, owned by it
    // (get rule), or null.
    let trust_ref: *mut c_void = unsafe { msg_send![&*space, serverTrust] };
    let trust = (!trust_ref.is_null()).then(|| {
        // SAFETY: a live, non-null `SecTrustRef`; the get rule retains it for the wrapper.
        unsafe { SecTrust::wrap_under_get_rule(trust_ref.cast()) }
    });
    let leaf = trust.as_ref().and_then(|trust| {
        let chain = trust.chain();
        chain
            .first()
            .map(|leaf| Fingerprint::of_certificate(&leaf.to_der()))
    });
    let host = space.host().to_string();
    let presented = Presented {
        host: &host,
        port: space.port(),
        leaf,
    };
    // Evaluated on the main thread, and only when the tab has no pin: see `rule`.
    let system_trust = || {
        trust
            .as_ref()
            .is_some_and(|trust| trust.evaluate_with_error().is_ok())
    };
    let disposition = disposition(decide(&presented, &system_trust), trust.is_some());
    if disposition != Disposition::UseCredential {
        return (disposition, None);
    }
    match trust.as_ref().and_then(credential) {
        Some(credential) => (Disposition::UseCredential, Some(credential)),
        None => (Disposition::CancelAuthenticationChallenge, None),
    }
}

/// `WebKit`'s disposition for an answer: accept only with a chain to accept, cancel, or the
/// system's own handling. Pure.
pub(super) fn disposition(answer: Answer, has_trust: bool) -> Disposition {
    match answer {
        Answer::Accept if has_trust => Disposition::UseCredential,
        Answer::Accept | Answer::Cancel => Disposition::CancelAuthenticationChallenge,
        Answer::Default => Disposition::PerformDefaultHandling,
    }
}

/// The credential that accepts `trust`'s chain.
fn credential(trust: &SecTrust) -> Option<Retained<NSURLCredential>> {
    let trust_ref = trust.as_concrete_TypeRef().cast::<c_void>();
    // SAFETY: `credentialForTrust:` takes a live `SecTrustRef` (held by `trust`) and returns an
    // autoreleased credential, retained here, or nil.
    unsafe { msg_send![NSURLCredential::class(), credentialForTrust: trust_ref] }
}
