import Foundation
import UIKit

@_silgen_name("wp_ios_signed_capabilities")
func signedCapabilities() -> UInt32
@_silgen_name("wp_ios_telephony_complete")
func nativeCompletion(_ identifier: UInt64, _ success: Int32, _ text: UnsafePointer<CChar>)
@_silgen_name("wp_ios_telephony_event")
func nativeEvent(_ text: UnsafePointer<CChar>)
@_silgen_name("wp_ios_device_dirty")
func nativeSnapshotChanged()
@_silgen_name("wp_ios_notification_policy")
func nativeNotificationPolicy() -> UInt32

struct CarrierNotificationPolicy {
    let bits: UInt32
    var enabled: Bool { bits & 1 != 0 && bits & 2 != 0 }
    var banners: Bool { bits & 4 != 0 }
    var sound: Bool { bits & 8 != 0 }
    var previews: Bool { bits & 32 != 0 }
    // iOS owns background notification vibration; no custom background haptic
    // operation is requested by this adapter.
}

func boundedText(_ text: String, _ bytes: Int, multiline: Bool = false) -> String {
    var result = ""
    var length = 0
    for scalar in text.unicodeScalars {
        let piece = CharacterSet.controlCharacters.contains(scalar) && !(multiline && (scalar == "\n" || scalar == "\t")) ? " " : String(scalar)
        if length + piece.utf8.count > bytes { break }
        result += piece; length += piece.utf8.count
    }
    return result
}

func response(_ kind: String, _ value: Any) -> [String: Any] { ["kind": kind, "value": value] }

@MainActor
func complete(_ id: UInt64, _ value: [String: Any]) {
    do {
        let data = try JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
        guard data.count <= 8 * 1024 * 1024, let text = String(data: data, encoding: .utf8) else {
            throw ProviderError("The native telephony response exceeds its data limit.")
        }
        text.withCString { nativeCompletion(id, 1, $0) }
    } catch { fail(id, error) }
}

@MainActor
func fail(_ id: UInt64, _ error: Error) {
    boundedText(error.localizedDescription, 4096, multiline: true).withCString { nativeCompletion(id, 0, $0) }
}

@MainActor
func emit(_ kind: String, _ value: Any) {
    guard let data = try? JSONSerialization.data(withJSONObject: response(kind, value), options: [.sortedKeys]),
          data.count <= 8 * 1024 * 1024, let text = String(data: data, encoding: .utf8) else { return }
    text.withCString { nativeEvent($0) }
}

struct ProviderError: LocalizedError {
    let message: String
    init(_ message: String) { self.message = message }
    var errorDescription: String? { message }
}

@MainActor
@_cdecl("wp_ios_telephony_snapshot")
public func telephonySnapshot() -> UnsafeMutablePointer<CChar>? {
    let value: [String: Any]
    if #available(iOS 26, *) { value = TelephonyProvider.shared.snapshot() }
    else { value = ["roles": [
        ["role": "dialer", "status": "unsupported", "detail": "The entitled dialer provider requires iOS 26 or later."],
        ["role": "sms", "status": "unsupported", "detail": "The entitled carrier provider requires iOS 26 or later."]]] }
    guard let data = try? JSONSerialization.data(withJSONObject: value), let text = String(data: data, encoding: .utf8) else { return nil }
    return text.withCString { strdup($0) }
}

@MainActor
@_cdecl("wp_ios_telephony_command")
public func telephonyCommand(_ id: UInt64, _ command: UnsafePointer<CChar>?) -> Int32 {
    guard #available(iOS 26, *), let command,
          let data = String(cString: command).data(using: .utf8), data.count <= 1024 * 1024,
          let request = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] else { return 0 }
    return TelephonyProvider.shared.start(id, request) ? 1 : 0
}

@MainActor
@_cdecl("wp_ios_telephony_cancel")
public func telephonyCancel(_ id: UInt64) {
    if #available(iOS 26, *) { TelephonyProvider.shared.cancel(id) }
}

@MainActor
@_cdecl("wp_ios_telephony_monitor")
public func telephonyMonitor(_ enabled: Int32) {
    if #available(iOS 26, *) { TelephonyProvider.shared.monitor(enabled) }
}
