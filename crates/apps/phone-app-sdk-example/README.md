# SDK counter

A statically linked phone app built with FerroUI and `phone-sdk`.

- App identity: `example.sdk-counter`.
- Entry point: `registration()`.
- Data: a versioned counter in its own `AppStorage` namespace.
- Host capabilities: none requested.

Add this crate to the composition dependencies, then pass
`phone_app_sdk_example::registration()` to
`phone_ui::create_view_with_apps`. See the workspace's `docs/sdk.md`.
