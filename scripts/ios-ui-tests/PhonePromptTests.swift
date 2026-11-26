import Foundation
import UIKit
import XCTest

/// Sends a fixed set of real system URLs on our disposable simulator.
/// The shell independently verifies the phone's delegate receipt and mounted UI.
final class PhonePromptTests: XCTestCase {
    private struct OpenRequest: Decodable {
        let sequence: Int
        let uri: String
    }
    private struct VerifiedRequest: Decodable {
        let sequence: Int
    }
    private struct NavigationRequest: Decodable {
        let sequence: Int
        let control: String
    }

    @MainActor
    private func navigationInk(_ button: XCUIElement, sequence: Int, directory: URL) throws -> [String: Any] {
        let screenshot = button.screenshot()
        let path = directory.appendingPathComponent("navigation-\(sequence)-button.png")
        try screenshot.pngRepresentation.write(to: path, options: .atomic)
        let image = try XCTUnwrap(screenshot.image.cgImage)
        XCTAssertGreaterThan(image.width, 0)
        XCTAssertGreaterThan(image.height, 0)
        XCTAssertLessThanOrEqual(image.width * image.height, 2_000_000)
        let rowBytes = image.width * 4
        var pixels = [UInt8](repeating: 0, count: rowBytes * image.height)
        try pixels.withUnsafeMutableBytes { bytes in
            let context = try XCTUnwrap(CGContext(
                data: bytes.baseAddress, width: image.width, height: image.height,
                bitsPerComponent: 8, bytesPerRow: rowBytes, space: CGColorSpaceCreateDeviceRGB(),
                bitmapInfo: CGBitmapInfo.byteOrder32Big.rawValue | CGImageAlphaInfo.premultipliedLast.rawValue
            ))
            context.draw(image, in: CGRect(x: 0, y: 0, width: CGFloat(image.width), height: CGFloat(image.height)))
        }
        var bright = 0
        for offset in stride(from: 0, to: pixels.count, by: 4) {
            if pixels[offset] > 100 && pixels[offset + 1] > 100 && pixels[offset + 2] > 100 && pixels[offset + 3] > 100 {
                bright += 1
            }
        }
        XCTAssertGreaterThanOrEqual(bright, 24, "The real navigation button screenshot must contain its visible glyph.")
        XCTAssertLessThan(bright, image.width * image.height / 2, "The navigation glyph must be distinct from its dark background.")
        let frame = button.frame
        return [
            "bright_opaque_pixels": bright,
            "screenshot": path.lastPathComponent,
            "image_width": image.width, "image_height": image.height,
            "frame": ["x": frame.minX, "y": frame.minY, "width": frame.width, "height": frame.height]
        ]
    }

    @MainActor
    func testOpenWindowsPhoneURLs() throws {
        continueAfterFailure = false
        let directory = try FileManager.default.url(
            for: .documentDirectory, in: .userDomainMask,
            appropriateFor: nil, create: true
        )
        let status = directory.appendingPathComponent("phone-prompt-status.json")
        let finished = directory.appendingPathComponent("phone-prompt-finished")
        let request = directory.appendingPathComponent("phone-url-request.json")
        let verified = directory.appendingPathComponent("phone-url-verified.json")
        let navigationRequest = directory.appendingPathComponent("phone-navigation-request.json")
        try? FileManager.default.removeItem(at: finished)
        try? FileManager.default.removeItem(at: request)
        try? FileManager.default.removeItem(at: verified)
        try? FileManager.default.removeItem(at: navigationRequest)
        let allowed = [
            "ferroui-phone://app/org.ferroui.wallet",
            "ferroui-phone://app/phone.phone?path=dial/KzEgMjAyIDU1NSAwMTQ3",
            "ferroui-phone://app/phone.people?path=device",
            "ferroui-phone://app/phone.messaging?path=compose",
            "ferroui-phone://app/phone.settings?path=device",
            "ferroui-phone://home"
        ]
        var accepted = 0
        var opened: [String] = []
        var requestedURI = ""
        var awaitingConfirmation = false
        let navigationOrder = ["HardwareSearch", "HardwareBack", "HardwareSearch", "HardwareStart"]
        var navigation: [[String: Any]] = []
        func publish(_ phase: String) throws {
            let value: [String: Any] = [
                "phase": phase, "accepted": accepted, "opened": opened,
                "requested_uri": requestedURI,
                "driver": "XCUIDevice.shared.system.open",
                "navigation_driver": "XCUIElement.tap",
                "navigation": navigation,
                "observed_at_unix_ms": Int64(Date().timeIntervalSince1970 * 1000)
            ]
            try JSONSerialization.data(withJSONObject: value).write(to: status, options: .atomic)
        }

        let phone = XCUIApplication(bundleIdentifier: "org.ferroui.windowsphone")
        XCTAssertNotEqual(phone.state, .notRunning, "The native smoke process must already be running.")
        // activate() preserves the existing process; launch() would replace it.
        phone.activate()
        XCTAssertTrue(phone.wait(for: .runningForeground, timeout: 20))
        let springboard = XCUIApplication(bundleIdentifier: "com.apple.springboard")
        let alert = springboard.alerts.containing(
            .staticText, identifier: "Open in “Windows Phone”?"
        ).firstMatch
        try publish("ready")
        let deadline = Date().addingTimeInterval(300)
        while !FileManager.default.fileExists(atPath: finished.path) && Date() < deadline {
            if awaitingConfirmation, let data = try? Data(contentsOf: verified),
               let acknowledgment = try? JSONDecoder().decode(VerifiedRequest.self, from: data),
               acknowledgment.sequence == opened.count {
                awaitingConfirmation = false
            }
            if FileManager.default.fileExists(atPath: request.path) {
                let bytes = try Data(contentsOf: request)
                XCTAssertLessThanOrEqual(bytes.count, 1024)
                let command = try JSONDecoder().decode(OpenRequest.self, from: bytes)
                XCTAssertLessThan(opened.count, allowed.count)
                guard opened.count < allowed.count else { return }
                XCTAssertEqual(command.sequence, opened.count + 1)
                XCTAssertEqual(command.uri, allowed[opened.count])
                guard command.sequence == opened.count + 1, command.uri == allowed[opened.count] else { return }
                try FileManager.default.removeItem(at: request)
                requestedURI = command.uri
                awaitingConfirmation = true
                try publish("opening")
                // This is Apple's system URL delivery API. It does not call
                // the phone's router. The independent verifier requires the
                // original phone PID/launch UUID and the actual delegate URI.
                // XCUISystem is a test-tool API introduced in Xcode 16.3;
                // Apple's availability metadata adds no newer iOS minimum.
                XCUIDevice.shared.system.open(try XCTUnwrap(URL(string: command.uri)))
                opened.append(command.uri)
                try publish("ready")
            }
            if awaitingConfirmation && alert.exists {
                let open = alert.buttons["Open"]
                XCTAssertTrue(open.waitForExistence(timeout: 5))
                XCTAssertTrue(open.isHittable)
                XCTAssertLessThan(accepted, 6, "Only the six named app/Home URL transitions are expected.")
                let capture = XCTAttachment(screenshot: springboard.screenshot())
                capture.name = "Windows Phone URL confirmation \(accepted + 1)"
                capture.lifetime = .keepAlways
                add(capture)
                open.tap()
                accepted += 1
                let dismissed = XCTNSPredicateExpectation(
                    predicate: NSPredicate(format: "exists == false"), object: alert
                )
                XCTAssertEqual(XCTWaiter.wait(for: [dismissed], timeout: 10), .completed)
                awaitingConfirmation = false
                try publish("ready")
            }
            if FileManager.default.fileExists(atPath: navigationRequest.path) {
                let bytes = try Data(contentsOf: navigationRequest)
                XCTAssertLessThanOrEqual(bytes.count, 1024)
                let command = try JSONDecoder().decode(NavigationRequest.self, from: bytes)
                XCTAssertEqual(opened, allowed, "Navigation follows all six independently verified native URL activations.")
                XCTAssertFalse(awaitingConfirmation)
                XCTAssertLessThan(navigation.count, navigationOrder.count)
                guard navigation.count < navigationOrder.count else { return }
                XCTAssertEqual(command.sequence, navigation.count + 1)
                XCTAssertEqual(command.control, navigationOrder[navigation.count])
                guard command.sequence == navigation.count + 1, command.control == navigationOrder[navigation.count] else { return }
                try FileManager.default.removeItem(at: navigationRequest)
                let button = phone.buttons[command.control]
                let hittable = XCTNSPredicateExpectation(predicate: NSPredicate { _, _ in
                    button.exists && button.isHittable && button.isEnabled
                }, object: button)
                XCTAssertEqual(XCTWaiter.wait(for: [hittable], timeout: 15), .completed)
                XCTAssertEqual(button.identifier, command.control)
                XCTAssertTrue(phone.frame.contains(button.frame))
                var proof = try navigationInk(button, sequence: command.sequence, directory: directory)
                proof["sequence"] = command.sequence
                proof["control"] = command.control
                proof["label"] = button.label
                proof["tap_started_at_unix_ms"] = Int64(Date().timeIntervalSince1970 * 1000)
                try publish("tapping")
                // This sends native UIKit touch input at the actual accessible
                // button; neither the route nor its command is injected.
                button.tap()
                proof["tap_completed_at_unix_ms"] = Int64(Date().timeIntervalSince1970 * 1000)
                navigation.append(proof)
                try publish("ready")
            }
            Thread.sleep(forTimeInterval: 0.25)
        }
        XCTAssertTrue(FileManager.default.fileExists(atPath: finished.path), "The route verifier did not finish.")
        XCTAssertEqual(opened, allowed, "All six system URL requests must have completed in order.")
        XCTAssertEqual(navigation.compactMap { $0["control"] as? String }, navigationOrder,
                       "Search, Back, Search and Start must receive real native taps in order.")
        try publish("complete")
    }
}
