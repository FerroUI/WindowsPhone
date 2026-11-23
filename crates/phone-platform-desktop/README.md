# Desktop platform plugin

`DesktopPlatform::new(HostMode)` implements `phone_host::platform::PlatformPlugin`
without a FerroUI view, window, renderer or application dependency. The shared
native runtime owns the actual visual surface and the existing media, camera,
file, clipboard, notification and location adapters. Desktop app discovery,
application activation, settings navigation and session-role observation belong
to this plugin.

| Platform | App catalog and activation | Session role |
| --- | --- | --- |
| Linux | GIO `GAppInfo` discovery, display rules and `GDesktopAppInfo` activation | Verified Windows Phone labwc session-client chain, matching session markers and the live Wayland socket |
| Windows | Shell `FOLDERID_AppsFolder`, `IEnumShellItems`, then a freshly resolved item PIDL through `ShellExecuteExW` | Direct system `CustomShellHost.exe` or `eshell.exe` parent; availability uses the Shell Launcher licensing query |
| macOS | User-facing `.app` bundles in system, local and user Applications directories; `NSWorkspace` activation | No replacement shell or default HOME-app role |

Application mode and launcher mode share the same services. Launcher mode does
not register a default HOME application on a desktop OS. Shell mode must pass
the shared contract's `ShellSession` and `HostRole::Active` checks before the
runtime starts. A fullscreen window, environment marker alone or an eligible
Windows edition does not satisfy that check. A desktop plugin can also be used
by an embedded surface without acquiring shell policy.

## Request and lifetime boundaries

The plugin returns only application IDs and titles. Native target identities
remain in its private catalog. A launch request must match an ID from the last
successful catalog; malformed or unlisted IDs cannot select a command, path,
URL or command-line arguments. The OS entry is resolved again before launch.
GIO owns desktop-entry parsing, quoting, XDG precedence and D-Bus activation;
Windows activation uses an AppsFolder item, not a parsing string passed to an
arbitrary launcher. macOS activation uses the canonical bundle URL observed by
discovery and refuses a moved/symlink-replaced target.

Discovery runs on the existing bounded native system worker. The catalog has
at most 4,096 entries, IDs at most 1,024 bytes and titles at most 512 UTF-8 bytes.
Failed or older discovery completions do not partially replace a catalog.
Closing clears admitted targets and suppresses later discovery results.
macOS application activation starts on the UI thread and completes through
`NSWorkspace`; at most four callbacks are pending, and a dropped future cannot
free a callback context still owned by the OS. Neither successful Linux launch
acceptance nor Windows shell acceptance proves that the launched application's
startup completed or that its window became focused.

System-settings requests use a closed page enum. Windows uses official
`ms-settings:` pages. Linux selects installed GNOME/KDE settings programs with
fixed arguments. macOS opens System Settings and explicitly asks the person to
choose the requested section; it does not pretend an undocumented preference
URL is a stable public API. Services owned by other native adapters are not
advertised by this plugin.

## Hosting primitives already in pinned FerroUI

The desktop entry point uses native Win32 and macOS backends; on Linux,
`use_platform_detect` selects X11. The separate Wayland crate supplies
`use_wayland` and `use_wayland_with_fallback` plus application-ID, GL-profile,
existing-display-FD and external-loop configuration. Both X11 and Wayland
backends host client windows; they do not implement a desktop compositor or
window manager.

`EmbeddableControlRoot::with_impl` accepts a host-owned `ITopLevelImpl`.
`ILauncher`, storage providers, clipboard, dispatcher and lifetime contracts
remain the framework's reusable service boundaries. `ILauncher` launches a
file/URI default handler; it has no installed-application catalog contract.
LinuxFramebuffer supplies direct DRM/fbdev rendering with
`start_linux_direct`, which is a separate single-view hosting model. It does
not supply the compositor needed to display arbitrary desktop applications.
This plugin therefore does not present framebuffer output as a desktop shell.

## Validation

```sh
cargo test -p phone-platform-desktop
python3 -m unittest discover -s crates/phone-desktop/platform/tests -v
```

The Linux protocol test creates its own XDG directories in a temporary
directory and a fresh test process. It exercises the actual optional GIO
runtime's visibility rules, hidden higher-priority override, argument
preservation and stale-entry rejection using a harmless fixture executable.
It does not launch a person's installed applications or connect to their
session bus. The child test is ignored when invoked directly; its parent owns
the isolated environment. If GIO is absent, the test reports that the protocol
was not exercised. Other unit tests cover catalog admission, atomic
replacement, text bounds and fixed settings routes.

The packaging tests generate files and execute only the isolated fixture
helper/preview/guard paths. They never install a session, alter Windows policy,
or start a compositor. Physical desktop-session operation, platform focus
policy, distribution services, Windows Shell Launcher deployment, and macOS
application consent/activation require testing on the intended device.

## Setup and primary references

- [Linux session deployment](../phone-desktop/platform/linux/README.md)
- [Windows Shell Launcher deployment](../phone-desktop/platform/windows/README.md)
- [GIO application discovery](https://docs.gtk.org/gio/type_func.AppInfo.get_all.html)
- [GIO activation contract](https://docs.gtk.org/gio/method.AppInfo.launch.html)
- [Desktop entry specification](https://specifications.freedesktop.org/desktop-entry/latest-single/)
- [Windows AppsFolder identity](https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid)
- [Windows shell item enumeration](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ishellitem-bindtohandler)
- [Windows item activation](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-shellexecuteinfow)
- [Apple NSWorkspace application activation](https://developer.apple.com/documentation/appkit/nsworkspace/openapplication(at:configuration:completionhandler:))
