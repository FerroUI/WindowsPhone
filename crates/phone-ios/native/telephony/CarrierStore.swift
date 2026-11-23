import Foundation
import CryptoKit
import SQLite3

struct CarrierRecord: Codable, Sendable {
    var id: String
    var thread_id: String
    var address: String
    var body: String
    var timestamp: Int64
    var outgoing: Bool
    var read: Bool
    var status: String
    var subscription: String
    var attachment_count: UInt16
    var recipients: [String]
    var transport: String
    var source_id: String
    var client_id: String?
    var request_hash: String?
    var subject: String = ""
    var can_download: Bool = false

    var wire: [String: Any] {
        ["id": id, "thread_id": thread_id, "address": address, "body": body,
         "timestamp": timestamp, "outgoing": outgoing, "read": read, "status": status,
         "subscription": subscription, "attachment_count": attachment_count,
         "subject": subject, "can_download": can_download]
    }

    static func digest(_ value: Data) -> String {
        SHA256.hash(data: value).map { String(format: "%02x", $0) }.joined()
    }
    static func thread(_ subscription: String, _ recipients: [String]) -> String {
        let members = recipients.map { $0.filter { !$0.isWhitespace && !"()-".contains($0) } }.sorted()
        return "ios:" + digest(Data(([subscription] + members).joined(separator: "\0").utf8))
    }
}

struct CarrierInsertion: Sendable {
    let record: CarrierRecord
    let inserted: Bool
}

struct CarrierThread: Sendable {
    let message: CarrierRecord
    let unread: UInt32

    var wire: [String: Any] {
        ["id": message.thread_id, "recipients": message.recipients,
         "title": boundedText(message.recipients.joined(separator: ", "), 512),
         "snippet": boundedText(message.body, 4096, multiline: true), "timestamp": message.timestamp,
         "unread": unread, "has_attachments": message.attachment_count > 0]
    }
}

/// A separate durable carrier inbox. UI backup/restore never rewrites this
/// database. Raw incoming payload and query metadata commit in one transaction.
actor CarrierStore {
    static let shared = CarrierStore()
    private var database: OpaquePointer?
    private let transient = unsafeBitCast(-1, to: sqlite3_destructor_type.self)

    deinit { if let database { sqlite3_close_v2(database) } }

    private func connection() throws -> OpaquePointer {
        if let database { return database }
        let manager = FileManager.default
        let base = try manager.url(for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: true)
            .appendingPathComponent("FerroUIWindowsPhone", isDirectory: true).appendingPathComponent("carrier", isDirectory: true)
        try manager.createDirectory(at: base, withIntermediateDirectories: true, attributes: [
            .posixPermissions: 0o700, .protectionKey: FileProtectionType.completeUntilFirstUserAuthentication])
        var pointer: OpaquePointer?
        let result = sqlite3_open_v2(base.appendingPathComponent("messages.sqlite3").path, &pointer,
                                    SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE | SQLITE_OPEN_FULLMUTEX, nil)
        guard result == SQLITE_OK, let pointer else {
            if let pointer { sqlite3_close_v2(pointer) }
            throw ProviderError("The private carrier message store could not be opened.")
        }
        sqlite3_busy_timeout(pointer, 3000)
        database = pointer
        do {
            try execute("PRAGMA journal_mode=WAL")
            try execute("PRAGMA synchronous=FULL")
            try execute("CREATE TABLE IF NOT EXISTS messages (id TEXT PRIMARY KEY, thread_id TEXT NOT NULL, client_id TEXT UNIQUE, timestamp INTEGER NOT NULL, read_flag INTEGER NOT NULL, status TEXT NOT NULL, metadata BLOB NOT NULL, payload BLOB NOT NULL)")
            try execute("CREATE INDEX IF NOT EXISTS messages_thread ON messages(thread_id,timestamp DESC,id)")
            try execute("CREATE INDEX IF NOT EXISTS messages_unread ON messages(thread_id,read_flag)")
            // A process restart cannot establish whether an interrupted send
            // reached the carrier. Preserve uncertainty and never auto-resend.
            try execute("UPDATE messages SET status='unknown' WHERE status='sending'")
        } catch { database = nil; sqlite3_close_v2(pointer); throw error }
        return pointer
    }

    private func checked(_ result: Int32) throws {
        guard result == SQLITE_OK || result == SQLITE_DONE || result == SQLITE_ROW else {
            throw ProviderError("The private carrier message store could not complete the operation (SQLite \(result)).")
        }
    }
    private func execute(_ sql: String) throws { try checked(sqlite3_exec(try connection(), sql, nil, nil, nil)) }
    private func statement(_ sql: String) throws -> OpaquePointer {
        var result: OpaquePointer?
        try checked(sqlite3_prepare_v2(try connection(), sql, -1, &result, nil))
        guard let result else { throw ProviderError("The message store did not prepare its query.") }
        return result
    }
    private func bind(_ statement: OpaquePointer, _ index: Int32, _ value: String?) throws {
        if let value { try value.withCString { try checked(sqlite3_bind_text(statement, index, $0, -1, transient)) } }
        else { try checked(sqlite3_bind_null(statement, index)) }
    }
    private func bind(_ statement: OpaquePointer, _ index: Int32, _ value: Data) throws {
        guard value.count <= 128 * 1024 * 1024 else { throw ProviderError("The carrier payload exceeds the 128 MiB storage limit.") }
        try value.withUnsafeBytes { try checked(sqlite3_bind_blob(statement, index, $0.baseAddress, Int32($0.count), transient)) }
    }
    private func blob(_ statement: OpaquePointer, _ index: Int32, maximum: Int = 128 * 1024 * 1024) throws -> Data {
        let size = Int(sqlite3_column_bytes(statement, index))
        guard size >= 0, size <= maximum, let bytes = sqlite3_column_blob(statement, index) else {
            if size == 0 { return Data() }
            throw ProviderError("The stored carrier payload is invalid or too large.")
        }
        return Data(bytes: bytes, count: size)
    }
    private func record(_ statement: OpaquePointer) throws -> CarrierRecord {
        var value = try JSONDecoder().decode(CarrierRecord.self, from: blob(statement, 0, maximum: 128 * 1024))
        value.read = sqlite3_column_int(statement, 1) != 0
        if let status = sqlite3_column_text(statement, 2) { value.status = String(cString: status) }
        return value
    }

    func insert<T: Encodable & Sendable>(_ record: CarrierRecord, payload: T) throws -> CarrierInsertion {
        let metadata = try JSONEncoder().encode(record)
        let raw = try JSONEncoder().encode(payload)
        let query = try statement("INSERT OR IGNORE INTO messages(id,thread_id,client_id,timestamp,read_flag,status,metadata,payload) VALUES(?,?,?,?,?,?,?,?)")
        defer { sqlite3_finalize(query) }
        try bind(query, 1, record.id); try bind(query, 2, record.thread_id); try bind(query, 3, record.client_id)
        try checked(sqlite3_bind_int64(query, 4, record.timestamp))
        try checked(sqlite3_bind_int(query, 5, record.read ? 1 : 0)); try bind(query, 6, record.status)
        try bind(query, 7, metadata); try bind(query, 8, raw); try checked(sqlite3_step(query))
        if sqlite3_changes(try connection()) == 0 {
            let existing = try record.client_id.flatMap { try byClient($0) } ?? message(record.id)
            guard let existing, existing.request_hash == record.request_hash else {
                throw ProviderError("That message identifier already belongs to a different request.")
            }
            if existing.can_download && record.transport == "mms" {
                return CarrierInsertion(record: try replaceIncoming(existing.id, record: record, payload: payload), inserted: true)
            }
            return CarrierInsertion(record: existing, inserted: false)
        }
        return CarrierInsertion(record: record, inserted: true)
    }

    func byClient(_ client: String) throws -> CarrierRecord? {
        let query = try statement("SELECT metadata,read_flag,status FROM messages WHERE client_id=?")
        defer { sqlite3_finalize(query) }; try bind(query, 1, client)
        let result = sqlite3_step(query); try checked(result)
        return result == SQLITE_ROW ? try record(query) : nil
    }
    func message(_ id: String) throws -> CarrierRecord? {
        let query = try statement("SELECT metadata,read_flag,status FROM messages WHERE id=?")
        defer { sqlite3_finalize(query) }; try bind(query, 1, id)
        let result = sqlite3_step(query); try checked(result)
        return result == SQLITE_ROW ? try record(query) : nil
    }
    func payload(_ id: String) throws -> Data {
        let query = try statement("SELECT payload FROM messages WHERE id=?")
        defer { sqlite3_finalize(query) }; try bind(query, 1, id)
        let result = sqlite3_step(query); try checked(result)
        guard result == SQLITE_ROW else { throw ProviderError("The selected carrier message is no longer stored.") }
        return try blob(query, 0)
    }
    func replaceIncoming<T: Encodable & Sendable>(_ id: String, record: CarrierRecord, payload: T) throws -> CarrierRecord {
        guard let previous = try message(id) else { throw ProviderError("This MMS is no longer stored.") }
        if !previous.can_download {
            // An incoming callback may finish the same download while a
            // user-initiated receive is awaiting the framework. Keep one row.
            guard previous.transport == record.transport, previous.source_id == record.source_id else {
                throw ProviderError("This message no longer matches the pending MMS download.")
            }
            return previous
        }
        var value = record; value.id = id; value.timestamp = previous.timestamp; value.read = previous.read
        value.can_download = false
        let query = try statement("UPDATE messages SET thread_id=?,status='received',metadata=?,payload=? WHERE id=?")
        defer { sqlite3_finalize(query) }
        try bind(query, 1, value.thread_id); try bind(query, 2, JSONEncoder().encode(value))
        try bind(query, 3, JSONEncoder().encode(payload)); try bind(query, 4, id); try checked(sqlite3_step(query))
        return value
    }
    func status(_ id: String, _ status: String) throws {
        let query = try statement("UPDATE messages SET status=? WHERE id=?")
        defer { sqlite3_finalize(query) }; try bind(query, 1, status); try bind(query, 2, id)
        try checked(sqlite3_step(query))
    }
    func messages(_ thread: String, limit: Int) throws -> [CarrierRecord] {
        let query = try statement("SELECT metadata,read_flag,status FROM messages WHERE thread_id=? ORDER BY timestamp DESC,id DESC LIMIT ?")
        defer { sqlite3_finalize(query) }; try bind(query, 1, thread)
        try checked(sqlite3_bind_int(query, 2, Int32(min(500, max(1, limit)))))
        var records: [CarrierRecord] = []
        while true {
            let result = sqlite3_step(query); try checked(result)
            if result == SQLITE_DONE { break }
            records.append(try record(query))
        }
        return records.reversed()
    }
    func threads(limit: Int) throws -> [CarrierThread] {
        let query = try statement("SELECT m.metadata,m.read_flag,m.status,(SELECT COUNT(*) FROM messages u WHERE u.thread_id=m.thread_id AND u.read_flag=0) FROM messages m WHERE m.id=(SELECT t.id FROM messages t WHERE t.thread_id=m.thread_id ORDER BY t.timestamp DESC,t.id DESC LIMIT 1) ORDER BY m.timestamp DESC LIMIT ?")
        defer { sqlite3_finalize(query) }; try checked(sqlite3_bind_int(query, 1, Int32(min(500, max(1, limit)))))
        var threads: [CarrierThread] = []
        while true {
            let result = sqlite3_step(query); try checked(result)
            if result == SQLITE_DONE { break }
            let value = try record(query)
            threads.append(CarrierThread(message: value,
                unread: UInt32(clamping: sqlite3_column_int64(query, 3))))
        }
        return threads
    }
    func markRead(_ thread: String) throws {
        let query = try statement("UPDATE messages SET read_flag=1 WHERE thread_id=?")
        defer { sqlite3_finalize(query) }; try bind(query, 1, thread); try checked(sqlite3_step(query))
    }
    func delete(_ thread: String, ids: [String]) throws {
        guard !ids.isEmpty, ids.count <= 500 else { throw ProviderError("Invalid message deletion batch.") }
        try execute("BEGIN IMMEDIATE")
        do {
            let query = try statement("DELETE FROM messages WHERE thread_id=? AND id=?")
            defer { sqlite3_finalize(query) }
            for id in ids {
                try checked(sqlite3_reset(query)); try checked(sqlite3_clear_bindings(query))
                try bind(query, 1, thread); try bind(query, 2, id); try checked(sqlite3_step(query))
            }
            try execute("COMMIT")
        } catch { try? execute("ROLLBACK"); throw error }
    }
}
