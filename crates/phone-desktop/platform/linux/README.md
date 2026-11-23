# Opt-in Linux desktop session

The Linux host can run as a normal application or a fullscreen launcher in an
existing X11/Wayland desktop. The provided session adds Windows Phone as an
explicit login-screen choice under **labwc**, which supplies the Wayland
compositor, window management and application switching. Windows Phone remains
the session's home/launcher process. Selecting this session does not turn the
FerroUI Wayland client backend into a compositor.

## Existing desktop

```sh
cargo run -p phone-desktop -- --mode app
cargo run -p phone-desktop -- --mode launcher --backend wayland
```

GIO supplies the installed application catalog. It handles desktop-file IDs,
XDG precedence, display visibility, executable argument quoting and D-Bus
activation. The plugin's `LaunchApplication` accepts only identities returned
by the observed catalog. Native launch acceptance does not prove that another
application finished starting or gained focus.

## Generate and review a login-session package

Build and place the executable at its final installed location first. Then
generate a new staging directory; generation changes no system files:

```sh
python3 crates/phone-desktop/platform/linux/stage-session.py \
  --executable /opt/windowsphone/phone-desktop \
  --output /tmp/windowsphone-session-package
```

The result contains the `.desktop` session entry, session wrapper, literal
executable helper, labwc configuration, file hashes and explicit install/remove
scripts. Review those files. On the intended Linux device, the generated
`install.sh --install` requires administrator privileges and refuses to replace
existing destination files. Without `--install`, it prints a preview message
and changes nothing. Installation adds a choice; it does not select a default
session, sign anyone out, enable autologin or launch labwc.

Keep the generated package. Its `remove.sh --remove` checks the installed files
against the generated hashes before removing only those exact files. A changed
file is left for manual review. Removal does not terminate an active session.

## Session requirements and behavior

- Install labwc at `/usr/bin/labwc`, the pinned FerroUI runtime prerequisites,
  the GIO runtime and a display manager that reads `/usr/share/wayland-sessions`.
  The configured executable must be accessible to the selected login account.
- Use a labwc version supporting `--session`, `ForEach`, the default keyboard
  bindings and the configuration syntax linked below. Distribution packaging
  determines the supported versions and runtime libraries.
- The session reuses the display manager's user D-Bus session. If no bus was
  supplied, it uses `/usr/bin/dbus-run-session`. labwc updates the display
  activation environment on its normal DRM session path.
- Install/configure the services the device needs: notification daemon,
  PipeWire, camera/desktop portals and an appropriate portal backend,
  permission agent, audio service, network service and GeoClue. The adapter
  reports unavailable or denied services rather than manufacturing capability.
  This package does not silently install or grant any of those services.
- Select **Windows Phone** at the login screen. The wrapper refuses to run
  inside an existing `DISPLAY` or `WAYLAND_DISPLAY` desktop.

The wrapper starts labwc with `-S` and the generated session client. The client
executes `phone-desktop --mode shell --backend wayland`. The plugin checks the
session identifiers, live Wayland socket and actual labwc/session-client
process chain. Merely supplying `--mode shell` in a terminal fails the shared
host-mode validation.

**Alt+Tab** switches native applications using labwc. **Super+Space** raises and
focuses the existing Windows Phone window identified by
`com.ferroui.WindowsPhone`. It does not start another instance.
**Super+Shift+Escape** ends this login session. Closing the session client also
lets labwc terminate through its `--session` lifecycle. Unsaved work in other
apps remains subject to their normal session-close behavior.

FerroUI's X11 and Wayland paths are ordinary client backends. The optional
LinuxFramebuffer backend is a direct DRM/fbdev single-view host; arbitrary
desktop-app windows still require a display server/compositor. No framebuffer
shell mode is advertised by this package.

## Verification boundary

The repository tests session generation, executable quoting, non-destructive
preview, the existing-desktop guard and the home-window binding. GIO protocol
tests use isolated fixtures. A successful build does not verify a particular
display manager, graphics driver, labwc release, camera portal or audio device.
The generated login session has to be qualified on the deployment machine.

Primary protocol documentation:

- [labwc session startup and activation environment](https://labwc.github.io/labwc.1.html)
- [labwc configuration](https://labwc.github.io/labwc-config.5.html)
- [labwc conditional actions and app IDs](https://labwc.github.io/labwc-actions.5.html)
- [Desktop entry specification](https://specifications.freedesktop.org/desktop-entry/latest-single/)
