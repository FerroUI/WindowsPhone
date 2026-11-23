# Opt-in Windows Shell Launcher configuration

Ordinary application and launcher modes do not replace Explorer. Windows
Shell Launcher is a separate Windows feature for eligible **Enterprise,
Education and IoT Enterprise** editions. Its per-account policy starts a
custom shell at sign-in. Shell Launcher does not itself restrict access to
other applications or provide a kiosk security boundary.

The desktop plugin observes the actual direct system `CustomShellHost.exe` or
`eshell.exe` parent before reporting the shell role active. The licensing query
only reports potential availability. `--mode shell` launched from a normal
terminal is refused; fullscreen alone does not acquire the role.

## Generate the policy

Install the built Windows executable at its final location and create the
intended dedicated account through normal Windows administration. On any
machine with Python, generate a review directory:

```sh
python3 crates/phone-desktop/platform/windows/generate-shell-config.py \
  --executable 'C:\Program Files\WindowsPhone\phone-desktop.exe' \
  --account '.\PhoneUser' \
  --output shell-policy
```

The generator accepts a literal absolute local executable path. It does not
accept environment expansion, UNC paths, wildcards, quotes or command-line
fragments. XML serialization escapes account/path text.

The generated Shell Launcher v2 XML maps only the named account to
`phone-desktop --mode shell`. Its default profile remains
`%SystemRoot%\explorer.exe` for other accounts. It creates no autologon account.
Other applications retain normal windowed behavior; Shell Launcher manages
the shell's fullscreen presentation. Exit code zero restarts the shell;
nonzero startup errors use `DoNothing` to avoid an uncontrolled restart loop.

## Review, deploy and recover

`ShellLauncher.ps1` defaults to **Inspect**, which displays the generated XML
without changing policy. **Apply** and **Restore** are explicit operations.
Microsoft's device-scoped MDM Bridge requires an existing **LocalSystem**
deployment context. The script checks that context and does not elevate
itself, install a privileged helper, change the Winlogon registry directly,
configure accounts or sign users out. Deploy through your established device
management process; the referenced Microsoft documentation describes the
available CSP/provisioning options.

The apply operation uses `MDM_AssignedAccess.ShellLauncher`. It records the
exact previous value in an exclusively created recovery file before changing
the property. An existing policy requires the explicit `-ReplaceExisting`
switch after review because the XML replaces the complete Shell Launcher
policy. `-WhatIf` previews the mutation through normal PowerShell semantics.
The script leaves the separate Assigned Access `Configuration` property alone.

Restoration requires the same computer and recovery record, and refuses to
overwrite a policy changed since that deployment. Keep the recovery file and
a separate administrative sign-in route. Policy changes take effect on the
next sign-in. Recovery from a failed shell uses the device's normal secure
Windows administration/sign-out path; the application does not intercept
Ctrl+Alt+Delete or claim to replace Windows security surfaces.

## Verification boundary

Tests verify generated XML, scope, fallback, quoting, failure actions and path
admission. Rust target checks verify the Windows API types; native Windows CI
must link the actual application. Neither operation applies a Shell Launcher
policy. The eligible edition, installed optional feature, account policy,
actual `CustomShellHost` process relationship and app-switching experience
must be validated on the intended Windows deployment.

Primary Microsoft documentation:

- [Shell Launcher overview and edition requirements](https://learn.microsoft.com/en-us/windows/configuration/shell-launcher/)
- [Shell Launcher v2 XML](https://learn.microsoft.com/en-us/windows/configuration/shell-launcher/configuration-file)
- [Configuration, MDM Bridge context and lifecycle](https://learn.microsoft.com/en-us/windows/configuration/shell-launcher/configure)
- [Shell Launcher licensing query example](https://learn.microsoft.com/en-us/windows/configuration/shell-launcher/configure-wmi)
