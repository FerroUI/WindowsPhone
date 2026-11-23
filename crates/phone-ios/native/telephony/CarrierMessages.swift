import Foundation
import UIKit
import TelephonyMessagingKit
import UniformTypeIdentifiers
import UserNotifications

@available(iOS 26, *)
struct PendingMMS: Codable, Sendable {
    let cellularServiceID: CellularServiceID
    let messageID: MMSMessageID
}

@available(iOS 26, *)
extension TelephonyProvider {
    func carrierRequest(_ request: [String: Any]) async throws -> [String: Any] {
        let store = CarrierStore.shared
        switch request["kind"] as? String {
        case "messageThreads":
            return response("messageThreads", try await store.threads(limit: request["limit"] as? Int ?? 200).map(\.wire))
        case "messages":
            guard let thread = request["thread_id"] as? String else { throw ProviderError("A carrier thread is required.") }
            let records = try await store.messages(thread, limit: request["limit"] as? Int ?? 200)
            return response("messages", ["thread_id": thread, "messages": records.map(\.wire)])
        case "markMessagesRead":
            guard let thread = request["thread_id"] as? String else { throw ProviderError("A carrier thread is required.") }
            try await store.markRead(thread)
            await clearThreadNotifications(thread)
            emit("messageThreads", try await store.threads(limit: 200).map(\.wire))
            return response("accepted", ["message": "Stored carrier messages were marked as read."])
        case "deleteMessages":
            guard let thread = request["thread_id"] as? String, let ids = request["ids"] as? [String] else {
                throw ProviderError("A carrier thread and message identifiers are required.")
            }
            try await store.delete(thread, ids: ids)
            UNUserNotificationCenter.current().removeDeliveredNotifications(withIdentifiers: ids)
            emit("messageThreads", try await store.threads(limit: 200).map(\.wire))
            return response("messages", ["thread_id": thread, "messages": try await store.messages(thread, limit: 200).map(\.wire)])
        case "sendMessage", "sendMultimediaMessage":
            return try await sendCarrierMessage(request)
        case "messageAttachments":
            guard let id = request["message_id"] as? String else { throw ProviderError("A carrier message is required.") }
            return response("messageAttachments", ["message_id": id, "attachments": try await carrierAttachments(id)])
        case "downloadMessage":
            guard let id = request["message_id"] as? String, let stored = try await store.message(id), stored.can_download else {
                throw ProviderError("The selected MMS is not awaiting a download.")
            }
            let pending = try JSONDecoder().decode(PendingMMS.self, from: await store.payload(id))
            let message = try await TelephonyMessagingSession.shared.mmsService.receiveMessage(using: pending.cellularServiceID, messageID: pending.messageID)
            let replacement = try await store.replaceIncoming(id, record: mmsRecord(message), payload: message)
            emit("messageThreads", try await store.threads(limit: 200).map(\.wire))
            return response("messageDownloaded", ["message_id": id, "thread_id": replacement.thread_id])
        default: throw ProviderError("This carrier messaging operation is unavailable.")
        }
    }

    func selectedSubscription(_ requested: String?) throws -> CellularServiceState {
        let services = try TelephonyMessagingSession.shared.cellularServices
        if let requested {
            guard let service = services.first(where: { "sms:" + $0.id.description == requested }) else {
                throw ProviderError("The selected carrier messaging subscription is no longer available.")
            }
            return service
        }
        guard services.count == 1, let service = services.first else {
            throw ProviderError(services.isEmpty ? "No carrier messaging subscription is available." : "Choose a carrier messaging subscription before sending.")
        }
        return service
    }

    func sendCarrierMessage(_ request: [String: Any]) async throws -> [String: Any] {
        guard UIApplication.shared.applicationState == .active,
              let recipients = request["recipients"] as? [String], !recipients.isEmpty, recipients.count <= 20,
              let body = request["body"] as? String, let client = request["client_id"] as? String else {
            throw ProviderError("Open the app and supply the intended recipients before sending a carrier message.")
        }
        let requestHash = CarrierRecord.digest(try JSONSerialization.data(withJSONObject: request, options: [.sortedKeys]))
        if let previous = try await CarrierStore.shared.byClient(client) {
            guard previous.request_hash == requestHash else { throw ProviderError("This draft identifier was already used for different message content.") }
            // Never resend a cancelled/ambiguous request. The durable record
            // preserves the provider outcome for explicit user review.
            return existingSend(previous, client: client)
        }
        let selected = try selectedSubscription(request["subscription"] as? String)
        let session = TelephonyMessagingSession.shared
        let subscription = "sms:" + selected.id.description
        let source = UInt32.random(in: 1...UInt32.max)
        let attachments = request["attachments"] as? [String] ?? []
        let multimedia = request["kind"] as? String == "sendMultimediaMessage" || recipients.count > 1
        var record = CarrierRecord(id: "ios:" + UUID().uuidString, thread_id: CarrierRecord.thread(subscription, recipients),
            address: recipients.joined(separator: ", "), body: boundedText(body, 65536, multiline: true),
            timestamp: Int64(Date().timeIntervalSince1970 * 1000), outgoing: true, read: true, status: "sending",
            subscription: subscription, attachment_count: UInt16(attachments.count), recipients: recipients,
            transport: multimedia ? "mms" : "sms", source_id: String(source), client_id: client, request_hash: requestHash)
        record.address = boundedText(record.address, 256)
        record.subject = boundedText(request["subject"] as? String ?? "", 512)
        if multimedia {
            guard session.mmsService.isViable(for: selected.id) else { throw ProviderError("MMS is unavailable for the selected subscription.") }
            var parts: [MMSPartContent] = []
            if !body.isEmpty {
                parts.append(MMSPartContent(data: Data(body.utf8), contentType: .utf8PlainText,
                    contentID: UUID().uuidString, disposition: .inline, fileName: "message.txt"))
            }
            for id in attachments {
                let attachment = try nativeDraftAttachment(id)
                parts.append(MMSPartContent(data: attachment.data, contentType: UTType(mimeType: attachment.mime),
                    contentID: UUID().uuidString, disposition: .attachment, fileName: attachment.name))
            }
            let content = MMSContent(parts: parts, recipients: recipients.map { MMSHandle(phoneNumber: $0) }, subject: request["subject"] as? String)
            let message = MMSMessage(cellularServiceID: selected.id, messageID: MMSMessageID(rawValue: source), content: content)
            let reserved = try await CarrierStore.shared.insert(record, payload: message).record
            guard reserved.id == record.id else {
                return existingSend(reserved, client: client)
            }
            try await beforeSend(record)
            do { try await session.mmsService.sendMessage(message) }
            catch { try await persistFailure(record, error); throw error }
        } else {
            guard session.smsService.isViable(for: selected.id), let recipient = recipients.first else {
                throw ProviderError("SMS is unavailable for the selected subscription.")
            }
            let message = SMSMessage(cellularServiceID: selected.id, handle: SMSHandle(phoneNumber: recipient),
                messageID: SMSMessageID(rawValue: source), content: SMSContent(body: body))
            let reserved = try await CarrierStore.shared.insert(record, payload: message).record
            guard reserved.id == record.id else {
                return existingSend(reserved, client: client)
            }
            try await beforeSend(record)
            do { try await session.smsService.sendMessage(message) }
            catch { try await persistFailure(record, error); throw error }
        }
        // Persist transport acceptance even if the UI was closed during send.
        try await CarrierStore.shared.status(record.id, "sent")
        if !closed && !Task.isCancelled { emit("messageThreads", try await CarrierStore.shared.threads(limit: 200).map(\.wire)) }
        return response("messageSent", ["client_id": client, "id": record.id, "status": "sent"])
    }

    func existingSend(_ record: CarrierRecord, client: String) -> [String: Any] {
        if record.status == "failed" || record.status == "unknown" {
            return response("sendFailed", ["client_id": client,
                "message": record.status == "unknown"
                    ? "The previous send has an unknown outcome. Review the stored conversation before deliberately creating another send."
                    : "The previous send failed. Its record is preserved; this repeated request has not sent the message again."])
        }
        return response("messageSent", ["client_id": client, "id": record.id, "status": record.status])
    }

    func clearThreadNotifications(_ thread: String) async {
        let center = UNUserNotificationCenter.current()
        let delivered = await center.deliveredNotifications()
        center.removeDeliveredNotifications(withIdentifiers: delivered.filter {
            $0.request.content.userInfo["ferrouiCarrierThread"] as? String == thread
        }.map { $0.request.identifier })
    }

    func beforeSend(_ record: CarrierRecord) async throws {
        do {
            try Task.checkCancellation()
            guard !closed, carrierConfigured, UIApplication.shared.applicationState == .active else {
                throw ProviderError("The carrier role or foreground app changed before the message was handed to iOS.")
            }
        } catch {
            try await CarrierStore.shared.status(record.id, "failed")
            throw error
        }
    }

    func persistFailure(_ record: CarrierRecord, _ error: Error) async throws {
        try await CarrierStore.shared.status(record.id, error is CancellationError ? "unknown" : "failed")
        if !closed && !Task.isCancelled {
            emit("messageThreads", try await CarrierStore.shared.threads(limit: 200).map(\.wire))
            if let client = record.client_id { emit("sendFailed", ["client_id": client, "message": boundedText(error.localizedDescription, 4096, multiline: true)]) }
        }
    }

    func startReceiving() {
        guard incoming.isEmpty, carrierConfigured else { return }
        streamError = ""
        incoming = [
            Task { @MainActor in
                do {
                    let notifications = try TelephonyMessagingSession.shared.smsService.incomingMessageNotifications
                    for await event in notifications {
                        try await self.receiveSMS(event.message)
                        if Task.isCancelled || self.closed || !self.carrierConfigured { break }
                    }
                } catch { self.receiveError(error) }
            },
            Task { @MainActor in
                do {
                    let notifications = try TelephonyMessagingSession.shared.mmsService.incomingMessageNotifications
                    for await event in notifications {
                        if #available(iOS 26.1, *) { try await self.receiveMMS(event.message) }
                        else {
                            do {
                                let message = try await TelephonyMessagingSession.shared.mmsService.receiveMessage(using: event.cellularServiceID, messageID: event.messageID)
                                try await self.receiveMMS(message)
                            } catch {
                                try await self.preservePendingMMS(PendingMMS(cellularServiceID: event.cellularServiceID, messageID: event.messageID))
                            }
                        }
                        if Task.isCancelled || self.closed || !self.carrierConfigured { break }
                    }
                } catch { self.receiveError(error) }
            },
            Task { @MainActor in
                do {
                    let notifications = try TelephonyMessagingSession.shared.rcsService.incomingMessageNotifications
                    for await event in notifications {
                        try await self.receiveRCS(event.message)
                        if Task.isCancelled || self.closed || !self.carrierConfigured { break }
                    }
                } catch { self.receiveError(error) }
            }
        ]
    }

    func incomingRecord(transport: String, service: CellularServiceID, source: String, recipients: [String], body: String, attachments: Int) -> CarrierRecord {
        let subscription = "sms:" + service.description
        let people = Array(Set(recipients.map { boundedText($0, 256) }.filter { !$0.isEmpty })).sorted().prefix(32)
        let addresses = Array(people)
        let identity = [transport, subscription, source].joined(separator: "\0")
        return CarrierRecord(id: "ios:" + CarrierRecord.digest(Data(identity.utf8)),
            thread_id: CarrierRecord.thread(subscription, addresses), address: boundedText(addresses.joined(separator: ", "), 256),
            body: boundedText(body, 65536, multiline: true), timestamp: Int64(Date().timeIntervalSince1970 * 1000),
            outgoing: false, read: false, status: "received", subscription: subscription,
            attachment_count: UInt16(min(100, max(0, attachments))), recipients: addresses,
            transport: transport, source_id: source, client_id: nil, request_hash: nil)
    }

    func receiveSMS(_ message: SMSMessage) async throws {
        let record = incomingRecord(transport: "sms", service: message.cellularServiceID, source: message.messageID.description,
            recipients: [message.handle.phoneNumber], body: message.content.body, attachments: 0)
        let result = try await CarrierStore.shared.insert(record, payload: message)
        if result.inserted { try await received(result.record) }
    }
    func mmsRecord(_ message: MMSMessage) -> CarrierRecord {
        let texts = message.content.parts.compactMap { part -> String? in
            guard !isMessageAttachment(part) else { return nil }
            return String(data: part.data, encoding: .utf8)
        }
        let attachments = message.content.parts.filter { isMessageAttachment($0) }.count
        var recipients = message.content.recipients.map(\.phoneNumber)
        if let sender = message.content.from?.phoneNumber { recipients.append(sender) }
        let body = texts.joined(separator: "\n")
        var record = incomingRecord(transport: "mms", service: message.cellularServiceID, source: message.messageID.description,
            recipients: recipients, body: body, attachments: attachments)
        record.subject = boundedText(message.content.subject ?? "", 512)
        return record
    }
    func receiveMMS(_ message: MMSMessage) async throws {
        let record = mmsRecord(message)
        let result = try await CarrierStore.shared.insert(record, payload: message)
        if result.inserted { try await received(result.record) }
    }
    func preservePendingMMS(_ pending: PendingMMS) async throws {
        var record = incomingRecord(transport: "mms", service: pending.cellularServiceID, source: pending.messageID.description,
            recipients: [], body: "Download this carrier message to read its sender, text and attachments.", attachments: 0)
        record.transport = "mms-pending" // Payload codec, not a different native message identity.
        record.thread_id = CarrierRecord.thread(record.subscription, ["pending:" + pending.messageID.description])
        record.subject = "MMS awaiting download"; record.can_download = true
        let result = try await CarrierStore.shared.insert(record, payload: pending)
        if result.inserted { try await received(result.record) }
    }
    func receiveRCS(_ message: RCSMessage) async throws {
        let body: String
        let attachments: Int
        switch message.content {
        case .text(let text): body = text.body; attachments = 0
        case .fileTransfer(let transfer): body = transfer.fileMetadata.fileName ?? "RCS attachment"; attachments = 1
        case .geolocationPush(let location): body = "\(location.description ?? "Location"): \(location.latitude), \(location.longitude)"; attachments = 0
        case .composingIndicator, .dispositionNotification: return
        default: body = "Rich carrier message"; attachments = 0
        }
        let record = incomingRecord(transport: "rcs", service: message.cellularServiceID, source: message.id.description,
            recipients: [message.handle.description], body: body, attachments: attachments)
        let result = try await CarrierStore.shared.insert(record, payload: message)
        if result.inserted { try await received(result.record) }
    }
    func received(_ record: CarrierRecord) async throws {
        guard !closed, !Task.isCancelled else { return }
        emit("messageThreads", try await CarrierStore.shared.threads(limit: 200).map(\.wire))
        guard UIApplication.shared.applicationState != .active else { return }
        let center = UNUserNotificationCenter.current()
        let settings = await center.notificationSettings()
        let policy = CarrierNotificationPolicy(bits: nativeNotificationPolicy())
        guard !closed, !Task.isCancelled, UIApplication.shared.applicationState != .active,
              policy.enabled,
              settings.authorizationStatus == .authorized || settings.authorizationStatus == .provisional else { return }
        let content = UNMutableNotificationContent()
        content.title = "New carrier message"
        content.subtitle = policy.previews ? record.address : ""
        content.body = policy.previews ? boundedText(record.body, 4096, multiline: true) : "Open Windows Phone to view the message."
        content.interruptionLevel = policy.banners ? .active : .passive
        if policy.banners && policy.sound && settings.soundSetting == .enabled { content.sound = .default }
        content.threadIdentifier = record.thread_id
        content.userInfo = ["ferrouiCarrierThread": record.thread_id]
        // A notification failure must not stop consuming and durably storing
        // subsequent carrier messages.
        do {
            try await center.add(UNNotificationRequest(identifier: record.id, content: content, trigger: nil))
            let now = CarrierNotificationPolicy(bits: nativeNotificationPolicy())
            if !now.enabled || (policy.previews && !now.previews) {
                center.removePendingNotificationRequests(withIdentifiers: [record.id])
                center.removeDeliveredNotifications(withIdentifiers: [record.id])
            }
        }
        catch { streamError = boundedText("Carrier notification failed: " + error.localizedDescription, 1024); nativeSnapshotChanged() }
    }
    func receiveError(_ error: Error) {
        guard !closed, !Task.isCancelled, !(error is CancellationError) else { return }
        streamError = boundedText("Carrier reception reported: " + error.localizedDescription, 1024)
        for task in incoming { task.cancel() }; incoming.removeAll()
        nativeSnapshotChanged()
    }
}
