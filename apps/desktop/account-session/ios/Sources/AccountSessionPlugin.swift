// The sign-in page of the Anima.ing account inside the app (mobile plan M3-5): the web authentication session of
// iOS, which shows the page over the app and shares the sign-in of Safari. The shell gives the address and gets the
// result of the sign-in itself, at a loopback address; this file only shows the page and takes it away.
// It logs nothing: the address holds the state of one sign-in (LOG-007).

import AuthenticationServices
import SwiftRs
import Tauri
import UIKit

struct OpenArgs: Decodable {
  let url: String
}

class AccountSessionPlugin: Plugin, ASWebAuthenticationPresentationContextProviding {
  private var session: ASWebAuthenticationSession?
  /// The call of `open` that waits for the page to go away.
  private var waiting: Invoke?

  /// Shows the page at `url`. It answers when the page is gone: the person closed it, or `close` took it away.
  @objc public func open(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(OpenArgs.self)
    guard let url = URL(string: args.url), url.scheme == "https" else {
      invoke.reject("url")
      return
    }
    DispatchQueue.main.async {
      if self.session != nil {
        invoke.reject("busy")
        return
      }
      // No callback scheme: the sign-in ends at a loopback address of the shell, and `close` ends the session.
      let session = ASWebAuthenticationSession(url: url, callbackURLScheme: nil) { _, _ in
        self.finish()
      }
      session.presentationContextProvider = self
      self.session = session
      self.waiting = invoke
      if !session.start() {
        self.session = nil
        self.waiting = nil
        invoke.reject("failed")
      }
    }
  }

  /// Takes the page away. With no page it does nothing.
  @objc public func close(_ invoke: Invoke) {
    DispatchQueue.main.async {
      self.session?.cancel()
      self.finish()
      invoke.resolve()
    }
  }

  /// Answers the waiting `open` one time. iOS does not always call the completion of a session that `cancel` ended.
  private func finish() {
    self.session = nil
    let waiting = self.waiting
    self.waiting = nil
    waiting?.resolve()
  }

  func presentationAnchor(for session: ASWebAuthenticationSession) -> ASPresentationAnchor {
    return self.manager.viewController?.view.window ?? ASPresentationAnchor()
  }
}

@_cdecl("init_plugin_account_session")
public func initPlugin() -> Plugin {
  return AccountSessionPlugin()
}
