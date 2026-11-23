import Foundation
import XCTest

/// Handles only the expected custom-scheme confirmation on our disposable simulator.
/// The shell harness independently issues URLs and verifies the phone's mounted UI.
final class PhonePromptTests: XCTestCase {
    @MainActor
    func testAcceptWindowsPhoneURLConfirmation() throws {
        continueAfterFailure = false
        let directory = try FileManager.default.url(
            for: .documentDirectory, in: .userDomainMask,
            appropriateFor: nil, create: true
        )
        let status = directory.appendingPathComponent("phone-prompt-status.json")
        let finished = directory.appendingPathComponent("phone-prompt-finished")
        try? FileManager.default.removeItem(at: finished)
        var accepted = 0
        func publish(_ phase: String) throws {
            let value: [String: Any] = ["phase": phase, "accepted": accepted]
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
            if alert.exists {
                let open = alert.buttons["Open"]
                XCTAssertTrue(open.waitForExistence(timeout: 5))
                XCTAssertTrue(open.isHittable)
                XCTAssertLessThan(accepted, 2, "Only the Wallet and Home confirmations are expected.")
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
                try publish("ready")
            }
            Thread.sleep(forTimeInterval: 0.25)
        }
        XCTAssertTrue(FileManager.default.fileExists(atPath: finished.path), "The route verifier did not finish.")
        XCTAssertGreaterThanOrEqual(accepted, 1, "The expected iOS scheme confirmation was never accepted.")
        try publish("complete")
    }
}
