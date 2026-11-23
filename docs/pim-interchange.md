# Contact and calendar interchange

People and Calendar exchange actual `.vcf` and `.ics` files with host applications. Native app-bar commands use the shared host request boundary; parsing, validation, selection, serialization and durable identifiers remain in reusable Rust domain/state modules.

People's **more → import contacts** reads one vCard file. **Save contacts copy** exports the current local list, up to 512 contacts. A contact card's **share contact** and **save contact copy** operate on that saved contact only. Calendar provides **import appointments**, **save calendar copy**, and appointment/series-specific sharing and copy commands. A recurring appointment explicitly says **share series** so a selected occurrence does not unexpectedly imply a standalone export.

## Supported records

| Format | Preserved content | Deliberate limits |
| --- | --- | --- |
| vCard 3.0/4.0 import; 4.0 export | UTF-8 display name, one telephone number, one email address and company. Folded lines, escaped separators and `tel:` URI decoding are supported. | The phone currently stores a display name, not structured name components or multiple labelled phone/email fields. Home/work and custom `TYPE` labels are rejected; generic mobile/voice and Internet types are accepted for the single supported field. Photos, postal addresses, birthdays, notes and other rich properties are rejected. Provider UID/revision metadata does not update an existing record. |
| iCalendar 2.0 VEVENT | Local wall-clock or all-day date, subject, duration, location, notes, availability and one display reminder relative to the start. | UTC/time-zone appointments, invitations/attendees, tasks, cancelled appointments, different reminder messages and audio/email reminders are rejected. This is a local calendar copy, not account synchronization. |
| Recurrence | Daily, weekdays, weekly, every two weeks, monthly or yearly; optional inclusive end date; matching excluded occurrence dates. | `COUNT`, arbitrary `BY*` rules, additional recurrence dates and detached overridden occurrences are rejected. Recurring copies retain their complete supported series. Weekday recurrence must start on a weekday; conflicting availability values and hidden date fields that cannot be represented in an exported copy are rejected. |

Every input is limited to 2 MB and 512 records, with bounded logical lines and properties. Existing model limits also apply: 20,000 contacts, 4,096 appointments/series, 512 exclusions per series, durations/reminder leads up to one week and the 16 MB phone document limit. Exported lines fold at 75 UTF-8 octets without splitting code points.

An import first parses the entire file, validates all records, reserves a checked contiguous identifier range and validates the complete candidate phone document. Only then does it publish the new data, refresh DynamicData projections and persist once. Failed parsing, unsupported content, capacity exhaustion and identifier exhaustion consume no identifiers and publish no partial import. Calendar imports invalidate foreground schedules once. A host observer cannot reenter an import; ordinary queued commands run after publication.

Imports create new local records. Reimporting a file creates another copy; it does not synchronize, merge or overwrite contacts/calendar items through an external identity. Favorites, contact colors and pinned tiles are local phone state, rather than personal information added to the exported vCard.

## Platform adapters

Browser imports use a user-initiated file picker. Exports download the actual Rust-generated file. Sharing builds the complete `File` synchronously and invokes `navigator.share` before an asynchronous boundary, preserving user activation. Capability rejection and cancellation never silently copy personal information to another channel.

Windows, macOS and Linux desktop builds use FerroUI's platform storage provider for open/save pickers. The desktop share action deliberately opens **Save a copy to share** and reports that the saved file can be attached in the user's chosen app. It does not claim to have shown a native share sheet or delivered a file to another application.

The portable codecs require neither a browser nor a desktop renderer. `phone-domain::interchange` exposes the byte parsers/exporters and `InterchangeFile`; the view model supplies atomic import and immutable selected-item export methods. The UI uses reusable native RSX menu helpers in `apps/pim.rs`.

## Verification scope

Focused Rust tests cover supported vCard and recurrence round trips, UTF-8 folding, all-day exclusive ends, overnight durations, malformed dates, rejected rich fields/time-zone semantics, atomic failures, identifier bounds, projection updates, schedule invalidation and reentrant observers. JavaScript host tests verify real file bytes, synchronous share invocation, cancellation and capability rejection with a mock sharing adapter.

`scripts/ui-pim-refinements.mjs` exercises the production Wasm byte callbacks and native People/Calendar sharing menus. It intentionally does not open the host file picker or share sheet, or send personal information. Those OS interactions and original Windows Phone visual equality require separate platform/device verification.

## Primary references

- [RFC 6350: vCard format](https://www.rfc-editor.org/rfc/rfc6350.html): UTF-8, content lines, folding, text escaping and contact properties.
- [RFC 2426: vCard 3.0](https://www.rfc-editor.org/rfc/rfc2426.html): supported legacy input representation.
- [RFC 5545: iCalendar](https://www.rfc-editor.org/rfc/rfc5545.html): date types, floating local time, exclusive event ends, recurrence/exclusions and display alarms.
- [Microsoft Exchange iCalendar busy-status mapping](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxcical/cd68eae7-ed65-4dd3-8ea7-ad585c76c736): `FREE`, `TENTATIVE`, `BUSY` and `OOF` interchange.
- [Microsoft: Contact Share on Nokia Lumia (2012)](https://blogs.windows.com/devices/2012/08/08/get-to-grips-with-contact-share-on-your-nokia-lumia/): original contact sharing behavior. This predates the targeted 8.1 Update release and is behavioral background, not evidence of exact 8.1 menu parity.

File import/export is a modern host integration for the supported local model. It does not reproduce legacy Microsoft-account contact/calendar synchronization or establish 100% People/Calendar feature parity.
