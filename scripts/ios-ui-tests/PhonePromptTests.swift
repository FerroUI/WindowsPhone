import Foundation
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
        try? FileManager.default.removeItem(at: finished)
        try? FileManager.default.removeItem(at: request)
        try? FileManager.default.removeItem(at: verified)
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
        func publish(_ phase: String) throws {
            let value: [String: Any] = [
                "phase": phase, "accepted": accepted, "opened": opened,
                "requested_uri": requestedURI,
                "driver": "XCUIDevice.shared.system.open",
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
            Thread.sleep(forTimeInterval: 0.25)
        }
        XCTAssertTrue(FileManager.default.fileExists(atPath: finished.path), "The route verifier did not finish.")
        XCTAssertEqual(opened, allowed, "All six system URL requests must have completed in order.")
        try publish("complete")
    }
}
