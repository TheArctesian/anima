import Foundation
import Security
import Tauri

private struct Entry: Decodable {
  let scope: String
  let account: String

  var query: [String: Any] {
    [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrService as String: scope,
      kSecAttrAccount as String: account,
      kSecAttrSynchronizable as String: false
    ]
  }
}

private struct Write: Decodable {
  let scope: String
  let account: String
  let token: String
}

extension AccountSessionPlugin {
  @objc public func readToken(_ invoke: Invoke) {
    guard let entry = try? invoke.parseArgs(Entry.self) else {
      invoke.reject("keystore")
      return
    }
    var query = entry.query
    query[kSecReturnData as String] = true
    query[kSecMatchLimit as String] = kSecMatchLimitOne
    var result: CFTypeRef?
    let status = SecItemCopyMatching(query as CFDictionary, &result)
    if status == errSecItemNotFound {
      invoke.resolve([String: String]())
    } else if status == errSecSuccess,
              let data = result as? Data,
              let token = String(data: data, encoding: .utf8) {
      invoke.resolve(["token": token])
    } else {
      invoke.reject("keystore")
    }
  }

  @objc public func writeToken(_ invoke: Invoke) {
    guard let args = try? invoke.parseArgs(Write.self), !args.token.isEmpty else {
      invoke.reject("keystore")
      return
    }
    let entry = Entry(scope: args.scope, account: args.account)
    let data = Data(args.token.utf8)
    var attributes = entry.query
    attributes[kSecValueData as String] = data
    attributes[kSecAttrAccessible as String] = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
    let added = SecItemAdd(attributes as CFDictionary, nil)
    let status = added == errSecDuplicateItem
      ? SecItemUpdate(entry.query as CFDictionary, [kSecValueData as String: data] as CFDictionary)
      : added
    if status == errSecSuccess { invoke.resolve() } else { invoke.reject("keystore") }
  }

  @objc public func clearToken(_ invoke: Invoke) {
    guard let entry = try? invoke.parseArgs(Entry.self) else {
      invoke.reject("keystore")
      return
    }
    let status = SecItemDelete(entry.query as CFDictionary)
    if status == errSecSuccess || status == errSecItemNotFound {
      invoke.resolve()
    } else {
      invoke.reject("keystore")
    }
  }
}
