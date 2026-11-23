import Foundation
import UIKit
import TelephonyMessagingKit
import UniformTypeIdentifiers

@_silgen_name("wp_ios_attachment_export")
func exportDraftAttachment(_ id: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>?
@_silgen_name("wp_ios_preview_carrier_attachment")
func nativeAttachmentPreview(_ id: UInt64, _ bytes: UnsafePointer<UInt8>, _ length: Int,
    _ name: UnsafePointer<CChar>, _ mime: UnsafePointer<CChar>) -> Int32

struct DraftAttachment {
    let name: String
    let mime: String
    let data: Data
}

@MainActor
func nativeDraftAttachment(_ id: String) throws -> DraftAttachment {
    guard let pointer = id.withCString({ exportDraftAttachment($0) }) else {
        throw ProviderError("A selected draft attachment expired. Select the file again.")
    }
    defer { free(pointer) }
    guard let json = String(cString: pointer).data(using: .utf8),
          let value = (try? JSONSerialization.jsonObject(with: json)) as? [String: Any],
          let name = value["name"] as? String, let mime = value["mime"] as? String,
          let content = value["data"] as? String, let data = Data(base64Encoded: content),
          !data.isEmpty, data.count <= 8 * 1024 * 1024 else { throw ProviderError("The selected draft attachment is invalid.") }
    return DraftAttachment(name: name, mime: mime, data: data)
}

@available(iOS 26, *)
func isMessageAttachment(_ part: MMSPartContent) -> Bool {
    part.disposition == .attachment || part.contentType?.conforms(to: .text) != true
}

@available(iOS 26, *)
extension TelephonyProvider {
    func carrierAttachments(_ id: String) async throws -> [[String: Any]] {
        guard let record = try await CarrierStore.shared.message(id) else { throw ProviderError("The selected carrier message no longer exists.") }
        let raw = try await CarrierStore.shared.payload(id)
        if record.transport == "mms" {
            let message = try JSONDecoder().decode(MMSMessage.self, from: raw)
            return message.content.parts.enumerated().filter { isMessageAttachment($0.element) }.prefix(100).map { index, part in
                ["id": "part:\(index)", "name": boundedText(part.filename.isEmpty ? "attachment-\(index + 1)" : part.filename, 255),
                 "mime": part.contentType?.preferredMIMEType ?? "application/octet-stream", "bytes": part.data.count]
            }
        }
        if record.transport == "rcs" {
            let message = try JSONDecoder().decode(RCSMessage.self, from: raw)
            if case .fileTransfer(let transfer) = message.content {
                let file = transfer.fileMetadata
                guard file.fileSize >= 0, file.fileSize <= 128 * 1024 * 1024 else { throw ProviderError("The RCS file metadata exceeds the attachment size limit.") }
                return [["id": "file", "name": boundedText(file.fileName ?? "RCS attachment", 255),
                         "mime": file.contentType?.preferredMIMEType ?? "application/octet-stream", "bytes": file.fileSize]]
            }
        }
        return []
    }

    func previewCarrierAttachment(_ operation: UInt64, _ request: [String: Any]) async throws {
        guard UIApplication.shared.applicationState == .active,
              let messageID = request["message_id"] as? String, let attachmentID = request["id"] as? String,
              let record = try await CarrierStore.shared.message(messageID) else {
            throw ProviderError("Open the app and choose a stored carrier attachment.")
        }
        let raw = try await CarrierStore.shared.payload(messageID)
        let content: Data
        let name: String
        let mime: String
        if record.transport == "mms" {
            let message = try JSONDecoder().decode(MMSMessage.self, from: raw)
            guard attachmentID.hasPrefix("part:"), let index = Int(attachmentID.dropFirst(5)),
                  message.content.parts.indices.contains(index), isMessageAttachment(message.content.parts[index]) else {
                throw ProviderError("The selected MMS attachment does not belong to this message.")
            }
            let part = message.content.parts[index]
            content = part.data; name = part.filename.isEmpty ? "attachment-\(index + 1)" : part.filename
            mime = part.contentType?.preferredMIMEType ?? "application/octet-stream"
        } else if record.transport == "rcs" {
            let message = try JSONDecoder().decode(RCSMessage.self, from: raw)
            guard attachmentID == "file", case .fileTransfer(let transfer) = message.content else {
                throw ProviderError("The selected RCS attachment does not belong to this message.")
            }
            let file = transfer.fileMetadata
            guard file.fileSize > 0, file.fileSize <= 24 * 1024 * 1024, file.expirationDate > Date() else {
                throw ProviderError("This RCS attachment expired or exceeds the 24 MiB preview limit.")
            }
            let folder = FileManager.default.temporaryDirectory.appendingPathComponent("FerroUICarrier-" + UUID().uuidString, isDirectory: true)
            try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: false,
                attributes: [.posixPermissions: 0o700, .protectionKey: FileProtectionType.completeUntilFirstUserAuthentication])
            defer { try? FileManager.default.removeItem(at: folder) }
            let destination = folder.appendingPathComponent("attachment")
            let download: RCSService.FileDownloadRequest
            if #available(iOS 26.5, *) {
                download = RCSService.FileDownloadRequest(cellularServiceID: message.cellularServiceID,
                    fileMetadata: file, destinationFileURL: destination)
            } else {
                download = RCSService.FileDownloadRequest(cellularServiceID: message.cellularServiceID,
                    fileURL: file.url, destinationFileURL: destination)
            }
            let result = try await TelephonyMessagingSession.shared.rcsService.download(download)
            try Task.checkCancellation()
            let attributes = try FileManager.default.attributesOfItem(atPath: destination.path)
            guard attributes[.type] as? FileAttributeType == .typeRegular,
                  let length = attributes[.size] as? NSNumber, length.int64Value > 0,
                  length.int64Value <= 24 * 1024 * 1024 else { throw ProviderError("The downloaded carrier attachment exceeds its file limit.") }
            content = try Data(contentsOf: destination)
            name = result.suggestedFileName ?? file.fileName ?? "RCS attachment"
            mime = result.contentType?.preferredMIMEType ?? file.contentType?.preferredMIMEType ?? "application/octet-stream"
        } else { throw ProviderError("This carrier message has no native attachments.") }
        try Task.checkCancellation()
        guard !content.isEmpty, content.count <= 24 * 1024 * 1024 else { throw ProviderError("The attachment exceeds the 24 MiB native preview limit.") }
        let started = content.withUnsafeBytes { bytes in
            name.withCString { name in mime.withCString { mime in
                nativeAttachmentPreview(operation, bytes.baseAddress!.assumingMemoryBound(to: UInt8.self), content.count, name, mime)
            }}
        }
        guard started == 1 else { throw ProviderError("The attachment preview was cancelled before presentation.") }
    }
}
