import Network
import SwiftRs
import Tauri
import WebKit

final class RunnerPlugin: Plugin {
  private let monitor = NWPathMonitor()
  private let pathQueue = DispatchQueue(label: "ing.anima.runner.path")

  override func load(webview: WKWebView) {
    monitor.start(queue: pathQueue)
  }

  deinit { monitor.cancel() }

  @objc public func readCapabilities(_ invoke: Invoke) {
    DispatchQueue.main.async {
      invoke.resolve(RunnerReport(path: self.monitor.currentPath))
    }
  }
}

@_cdecl("init_plugin_runner")
public func initPlugin() -> Plugin {
  return RunnerPlugin()
}
