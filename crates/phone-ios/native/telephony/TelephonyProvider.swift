import Foundation
import UIKit
import LiveCommunicationKit
import TelephonyMessagingKit

@available(iOS 26, *)
@MainActor
final class TelephonyProvider {
    static let shared = TelephonyProvider()
    var tasks: [UInt64: Task<Void, Never>] = [:]
    var incoming: [Task<Void, Never>] = []
    var historyTask: Task<Void, Never>?
    var generation: UInt64 = 0
    var closed = false
    var observingHistory = false
    var dialerActive = false
    var history: [[String: Any]] = []
    var streamError = ""
    var historyObservation: NotificationCenter.ObservationToken?

    var isPhone: Bool {
        #if targetEnvironment(simulator)
        return false
        #else
        return UIDevice.current.userInterfaceIdiom == .phone && !ProcessInfo.processInfo.isiOSAppOnMac
        #endif
    }
    var dialerEntitled: Bool { isPhone && signedCapabilities() & 1 != 0 }
    var carrierEntitled: Bool { isPhone && signedCapabilities() & 2 != 0 }
    var carrierConfigured: Bool { carrierEntitled && TelephonyMessagingSession.shared.isConfiguredForCarrierMessaging }

    func snapshot() -> [String: Any] {
        let dialer = dialerEntitled
        let carrier = carrierEntitled
        let configured = carrierConfigured
        let calls = dialer ? TelephonyConversationManager.sharedInstance.cellularServices : []
        let sms = configured ? ((try? TelephonyMessagingSession.shared.cellularServices) ?? []) : []
        var operations: [String] = []
        if dialer { operations.append("dial") }
        if dialerActive { operations.append("callHistory") }
        if configured { operations += ["readMessages", "deleteMessages", "messageAttachments", "downloadMessages"] }
        if configured && sms.contains(where: { TelephonyMessagingSession.shared.smsService.isViable(for: $0.id) || TelephonyMessagingSession.shared.mmsService.isViable(for: $0.id) }) {
            operations.append("sendMessages")
        }
        if configured && sms.contains(where: { TelephonyMessagingSession.shared.mmsService.isViable(for: $0.id) }) {
            operations += ["sendMultimediaMessages", "pickMessageAttachments"]
        }
        return [
            "roles": [
                ["role": "dialer", "status": dialerActive ? "active" : dialer ? "available" : "unsupported",
                 "detail": dialer ? "Signed Default Dialer entitlement detected. History access verifies the selected role; iOS and regional eligibility authorize each operation." : "The running device/build does not have an eligible signed Default Dialer entitlement. The standard calling handoff remains available."],
                ["role": "sms", "status": configured ? "active" : carrier ? "available" : "unsupported",
                 "detail": configured ? "TelephonyMessagingKit reports that this app is configured for carrier messaging. " + streamError : "Carrier messaging requires the signed entitlement, an eligible iPhone/region and selection as default. The normal message composer remains separate."]
            ],
            "permissions": [
                ["permission": "phone", "status": dialer ? "granted" : "unsupported"],
                ["permission": "callLog", "status": dialerActive ? "granted" : dialer ? "denied" : "unsupported"],
                ["permission": "readSms", "status": configured ? "granted" : carrier ? "denied" : "unsupported"],
                ["permission": "sendSms", "status": configured ? "granted" : carrier ? "denied" : "unsupported"]
            ],
            "phone_accounts": calls.prefix(16).map { ["id": "call:" + $0.id.uuidString, "label": boundedText($0.label.isEmpty ? "Cellular account" : $0.label, 512), "is_default": false] as [String: Any] },
            "sms_subscriptions": sms.prefix(16).map { ["id": "sms:" + $0.id.description, "label": boundedText($0.label.isEmpty ? "Carrier messaging" : $0.label, 512), "is_default": sms.count == 1] as [String: Any] },
            "operations": operations
        ]
    }

    func start(_ id: UInt64, _ request: [String: Any]) -> Bool {
        guard !closed, let kind = request["kind"] as? String else { return false }
        let dialerCommands = ["dial", "callHistory"]
        let carrierCommands = ["messageThreads", "messages", "sendMessage", "sendMultimediaMessage", "messageAttachments", "openMessageAttachment", "downloadMessage", "markMessagesRead", "deleteMessages"]
        if kind == "requestRole" {
            let role = request["role"] as? String
            guard (role == "dialer" && dialerEntitled) || (role == "sms" && carrierEntitled) else { return false }
        } else if dialerCommands.contains(kind) {
            guard dialerEntitled else { return false }
        } else if carrierCommands.contains(kind) {
            guard carrierEntitled else { return false }
        } else { return false }
        guard tasks.count < 16 else { fail(id, ProviderError("Too many telephony operations are pending.")); return true }
        let owner = generation
        tasks[id] = Task { @MainActor in
            defer { self.tasks.removeValue(forKey: id) }
            do {
                let value = try await self.perform(id, request)
                guard !Task.isCancelled, !self.closed, self.generation == owner else { return }
                if let value { complete(id, value) }
            } catch {
                guard !Task.isCancelled, !self.closed, self.generation == owner else { return }
                fail(id, error)
            }
        }
        return true
    }

    func cancel(_ id: UInt64) { tasks.removeValue(forKey: id)?.cancel() }

    func perform(_ identifier: UInt64, _ request: [String: Any]) async throws -> [String: Any]? {
        try Task.checkCancellation()
        switch request["kind"] as? String {
        case "requestRole":
            guard UIApplication.shared.applicationState == .active,
                  let url = URL(string: UIApplication.openDefaultApplicationsSettingsURLString) else {
                throw ProviderError("Open the app before choosing a default application.")
            }
            let accepted = await UIApplication.shared.open(url)
            guard accepted else { throw ProviderError("iOS could not open Default Apps settings.") }
            return response("accepted", ["message": "Default Apps settings opened. Selection and regional eligibility are verified when the app resumes."])
        case "dial":
            guard UIApplication.shared.applicationState == .active, let number = request["number"] as? String else {
                throw ProviderError("Open the app before starting a cellular call.")
            }
            let manager = TelephonyConversationManager.sharedInstance
            var account: LiveCommunicationKit.CellularService?
            if let selected = request["account"] as? String {
                guard let found = manager.cellularServices.first(where: { "call:" + $0.id.uuidString == selected }) else {
                    throw ProviderError("The selected cellular calling account is no longer available.")
                }
                account = found
            }
            let action = StartCellularConversationAction(Handle(type: .phoneNumber, value: number), cellularService: account)
            try await manager.startCellularConversation(action)
            return response("accepted", ["message": "iOS accepted the cellular conversation request. The carrier and system manage connection and in-call controls."])
        case "callHistory":
            let values = try await refreshHistory()
            return response("callHistory", Array(values.prefix(min(500, request["limit"] as? Int ?? 200))))
        default:
            guard carrierConfigured else { throw ProviderError("iOS has not configured this app as the eligible default carrier messaging app.") }
            if request["kind"] as? String == "openMessageAttachment" {
                try await previewCarrierAttachment(identifier, request)
                return nil // UIKit owns completion until its preview dismisses.
            }
            return try await carrierRequest(request)
        }
    }

    func refreshHistory() async throws -> [[String: Any]] {
        let predicate = #Predicate<ConversationHistoryManager.RecentConversation> { _ in true }
        do {
            let values = try await ConversationHistoryManager.sharedInstance.recentConversations(matching: predicate)
            try Task.checkCancellation()
            dialerActive = true
            history = values.sorted(by: { $0.date > $1.date }).prefix(500).map { call in
                ["id": call.id.uuidString, "number": boundedText(call.handles.first?.value ?? "", 256),
                 "name": "", "timestamp": Int64(call.date.timeIntervalSince1970 * 1000),
                 "duration": UInt32(max(0, min(call.duration, Double(UInt32.max)))),
                 "direction": call.status == .missed ? "missed" : call.direction == .incoming ? "incoming" : "outgoing"]
            }
            nativeSnapshotChanged()
            return history
        } catch {
            if !Task.isCancelled && !closed && observingHistory && !(error is CancellationError) {
                dialerActive = false; history = []; nativeSnapshotChanged()
            }
            throw error
        }
    }

    func monitor(_ enabled: Int32) {
        if enabled < 0 {
            closed = true; observingHistory = false; generation &+= 1
            for task in tasks.values { task.cancel() }; tasks.removeAll()
            for task in incoming { task.cancel() }; incoming.removeAll()
            historyTask?.cancel(); historyTask = nil; historyObservation = nil
            dialerActive = false; history = []
            return
        }
        if enabled == 0 {
            observingHistory = false
            historyTask?.cancel(); historyTask = nil
            // Carrier message streams remain registered during OS-granted
            // background execution. Their store is independent of UI delivery.
            return
        }
        closed = false; observingHistory = true
        if dialerEntitled {
            dialerActive = false
            historyTask?.cancel()
            historyTask = Task { @MainActor in
                do { _ = try await self.refreshHistory() } catch { /* observed in snapshot */ }
            }
            if historyObservation == nil {
                let observerGeneration = generation
                historyObservation = NotificationCenter.default.addObserver(of: ConversationHistoryManager.self, for: .conversationHistoryDidUpdateMessage) { [weak self] _ in
                    Task { @MainActor [weak self] in
                        guard let self, !self.closed, self.observingHistory,
                              self.generation == observerGeneration else { return }
                        self.historyTask?.cancel()
                        self.historyTask = Task { @MainActor in
                            do { let history = try await self.refreshHistory(); emit("callHistory", history) }
                            catch { /* permission and role are refreshed by the provider */ }
                        }
                    }
                }
            }
        }
        if carrierConfigured { startReceiving() }
        else { for task in incoming { task.cancel() }; incoming.removeAll() }
        nativeSnapshotChanged()
    }
}
