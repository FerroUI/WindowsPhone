//! GIO owns XDG precedence, visibility, desktop-entry parsing and D-Bus/Exec
//! activation. In particular, this adapter never evaluates an Exec line.
use super::{
    application, Application, HostFeature, HostMode, HostRole, PlatformState,
    MAX_DEVICE_APPLICATIONS,
};
use libloading::Library;
use phone_host::platform::{PlatformResponse, PlatformTask, SystemSettingsPage};
use std::{
    ffi::{c_char, c_int, c_void, CStr, CString},
    os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::OnceLock,
};

#[repr(C)]
struct List {
    data: *mut c_void,
    next: *mut List,
    previous: *mut List,
}
#[repr(C)]
struct Error {
    domain: u32,
    code: c_int,
    message: *const c_char,
}

struct Gio {
    // Keep every defining library loaded for the function pointers' lifetime.
    _gio: Library,
    _gobject: Library,
    _glib: Library,
    all: unsafe extern "C" fn() -> *mut List,
    id: unsafe extern "C" fn(*mut c_void) -> *const c_char,
    title: unsafe extern "C" fn(*mut c_void) -> *const c_char,
    visible: unsafe extern "C" fn(*mut c_void) -> c_int,
    hidden: unsafe extern "C" fn(*mut c_void) -> c_int,
    new: unsafe extern "C" fn(*const c_char) -> *mut c_void,
    launch: unsafe extern "C" fn(*mut c_void, *mut List, *mut c_void, *mut *mut Error) -> c_int,
    unref: unsafe extern "C" fn(*mut c_void),
    free_list: unsafe extern "C" fn(*mut List, Option<unsafe extern "C" fn(*mut c_void)>),
    free_error: unsafe extern "C" fn(*mut Error),
}

impl Gio {
    fn load() -> Result<Self, String> {
        // Fixed platform sonames, never supplied by an app or desktop entry.
        // The OS dynamic loader applies the process's normal library policy.
        unsafe {
            let gio = Library::new("libgio-2.0.so.0").map_err(|e| e.to_string())?;
            let gobject = Library::new("libgobject-2.0.so.0").map_err(|e| e.to_string())?;
            let glib = Library::new("libglib-2.0.so.0").map_err(|e| e.to_string())?;
            macro_rules! symbol {
                ($library:ident, $name:literal) => {
                    *$library
                        .get(concat!($name, "\0").as_bytes())
                        .map_err(|e| e.to_string())?
                };
            }
            Ok(Self {
                all: symbol!(gio, "g_app_info_get_all"),
                id: symbol!(gio, "g_app_info_get_id"),
                title: symbol!(gio, "g_app_info_get_display_name"),
                visible: symbol!(gio, "g_app_info_should_show"),
                hidden: symbol!(gio, "g_desktop_app_info_get_is_hidden"),
                new: symbol!(gio, "g_desktop_app_info_new"),
                launch: symbol!(gio, "g_app_info_launch"),
                unref: symbol!(gobject, "g_object_unref"),
                free_list: symbol!(glib, "g_list_free_full"),
                free_error: symbol!(glib, "g_error_free"),
                _gio: gio,
                _gobject: gobject,
                _glib: glib,
            })
        }
    }
}

static GIO: OnceLock<Result<Gio, String>> = OnceLock::new();
fn gio() -> Result<&'static Gio, String> {
    GIO.get_or_init(Gio::load)
        .as_ref()
        .map_err(|error| format!("The GIO application catalog is unavailable: {error}"))
}

struct OwnedList<'a> {
    list: *mut List,
    gio: &'a Gio,
}
impl Drop for OwnedList<'_> {
    fn drop(&mut self) {
        unsafe {
            (self.gio.free_list)(self.list, Some(self.gio.unref));
        }
    }
}
struct OwnedApp<'a> {
    value: *mut c_void,
    gio: &'a Gio,
}
impl Drop for OwnedApp<'_> {
    fn drop(&mut self) {
        unsafe {
            (self.gio.unref)(self.value);
        }
    }
}

unsafe fn text(value: *const c_char) -> Option<String> {
    if value.is_null() {
        return None;
    }
    // GLib's getter contract supplies a valid, NUL-terminated string owned by
    // the live GAppInfo. Copy it before releasing that reference.
    let value = unsafe { CStr::from_ptr(value) }.to_str().ok()?;
    (value.len() <= 16_384).then(|| value.to_owned())
}

fn valid_id(id: &str) -> bool {
    id.len() <= 1000
        && id.ends_with(".desktop")
        && id.len() > 8
        && !id.contains(['/', '\\'])
        && !id.chars().any(char::is_control)
}

pub(super) fn applications() -> Result<Vec<Application>, String> {
    let gio = gio()?;
    let list = OwnedList {
        list: unsafe { (gio.all)() },
        gio,
    };
    let mut current = list.list;
    let mut apps = Vec::new();
    while !current.is_null() {
        let entry = unsafe { &*current };
        current = entry.next;
        if entry.data.is_null() || unsafe { (gio.visible)(entry.data) } == 0 {
            continue;
        }
        let Some(id) = (unsafe { text((gio.id)(entry.data)) }).filter(|id| valid_id(id)) else {
            continue;
        };
        let Some(title) = (unsafe { text((gio.title)(entry.data)) }) else {
            continue;
        };
        if let Some(app) = application("linux", &id, &title, &id) {
            apps.push(app);
        }
        if apps.len() > MAX_DEVICE_APPLICATIONS {
            return Err("The installed application catalog exceeds its limit.".into());
        }
    }
    Ok(apps)
}

pub(super) fn launch(id: &str) -> Result<(), String> {
    if !valid_id(id) {
        return Err("Invalid desktop application identity.".into());
    }
    let gio = gio()?;
    let identity = CString::new(id).map_err(|_| "Invalid desktop application identity.")?;
    let value = unsafe { (gio.new)(identity.as_ptr()) };
    if value.is_null() {
        return Err(
            "That application is no longer installed. Refresh the device applications.".into(),
        );
    }
    let app = OwnedApp { value, gio };
    if unsafe { (gio.visible)(app.value) } == 0 || unsafe { (gio.hidden)(app.value) } != 0 {
        return Err("That application is no longer visible in this desktop session.".into());
    }
    let mut error = std::ptr::null_mut();
    let accepted = unsafe {
        (gio.launch)(
            app.value,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut error,
        )
    };
    let message = if error.is_null() {
        None
    } else {
        let message = unsafe { text((*error).message) };
        unsafe {
            (gio.free_error)(error);
        }
        message
    };
    if accepted == 0 {
        Err(message.unwrap_or_else(|| "The desktop refused this application launch.".into()))
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Settings {
    Gnome(&'static str),
    Kde(&'static str),
}
fn executable(path: &str) -> bool {
    std::fs::metadata(path)
        .is_ok_and(|value| value.is_file() && value.permissions().mode() & 0o111 != 0)
}
fn settings() -> Option<Settings> {
    let gnome = [
        "/usr/bin/gnome-control-center",
        "/usr/local/bin/gnome-control-center",
    ]
    .into_iter()
    .find(|path| executable(path))
    .map(Settings::Gnome);
    let kde = [
        "/usr/bin/systemsettings",
        "/usr/bin/systemsettings5",
        "/usr/local/bin/systemsettings",
    ]
    .into_iter()
    .find(|path| executable(path))
    .map(Settings::Kde);
    if std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .split(':')
        .any(|name| name.eq_ignore_ascii_case("KDE"))
    {
        kde.or(gnome)
    } else {
        gnome.or(kde)
    }
}
fn settings_command(
    settings: Settings,
    page: SystemSettingsPage,
) -> (&'static str, &'static [&'static str]) {
    match (settings, page) {
        (Settings::Gnome(program), SystemSettingsPage::Application) => (program, &["applications"]),
        (Settings::Gnome(program), SystemSettingsPage::Network) => (program, &["network"]),
        (Settings::Gnome(program), SystemSettingsPage::Notifications) => {
            (program, &["notifications"])
        }
        (Settings::Gnome(program), SystemSettingsPage::DefaultApps) => (program, &["default-apps"]),
        (Settings::Kde(program), SystemSettingsPage::Application) => (program, &[]),
        (Settings::Kde(program), SystemSettingsPage::Network) => {
            (program, &["kcm_networkmanagement"])
        }
        (Settings::Kde(program), SystemSettingsPage::Notifications) => {
            (program, &["kcm_notifications"])
        }
        (Settings::Kde(program), SystemSettingsPage::DefaultApps) => {
            (program, &["kcm_componentchooser"])
        }
        (Settings::Gnome(program), page) => (
            program,
            match page {
                SystemSettingsPage::Wifi => &["wifi"],
                SystemSettingsPage::Bluetooth => &["bluetooth"],
                SystemSettingsPage::Display
                | SystemSettingsPage::Brightness
                | SystemSettingsPage::Rotation
                | SystemSettingsPage::NightLight => &["display"],
                SystemSettingsPage::Sound | SystemSettingsPage::AudioOutput => &["sound"],
                SystemSettingsPage::Battery
                | SystemSettingsPage::BatterySaver
                | SystemSettingsPage::ScreenTimeout => &["power"],
                SystemSettingsPage::Accounts | SystemSettingsPage::Sync => &["online-accounts"],
                SystemSettingsPage::DateTime => &["datetime"],
                SystemSettingsPage::Language => &["region"],
                SystemSettingsPage::Keyboard => &["keyboard"],
                SystemSettingsPage::Accessibility => &["universal-access"],
                SystemSettingsPage::About => &["info-overview"],
                SystemSettingsPage::Hotspot
                | SystemSettingsPage::Vpn
                | SystemSettingsPage::Cellular => &["network"],
                _ => &[],
            },
        ),
        (Settings::Kde(program), _) => (program, &[]),
    }
}
pub(super) fn open_settings(page: SystemSettingsPage) -> PlatformTask {
    Box::pin(async move {
        phone_host::native::background::run_system(move || {
            let settings = settings()
                .ok_or("No supported system-settings application is installed (GNOME or KDE).")?;
            let (program, args) = settings_command(settings, page);
            let mut child = Command::new(program)
                .args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|error| format!("Could not open system settings: {error}"))?;
            // This is an application, not a short helper: never wait for its UI
            // to close on the host worker. Reap it independently.
            std::thread::Builder::new()
                .name("phone-settings".into())
                .spawn(move || {
                    let _ = child.wait();
                })
                .map_err(|error| error.to_string())?;
            Ok(())
        })
        .await?;
        Ok(PlatformResponse::Accepted { message: "The system-settings application was started. Its installed panels determine which page is available.".into() })
    })
}

fn read_bounded(path: &Path, limit: usize) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() <= limit).then_some(bytes)
}
fn parent_pid(stat: &str) -> Option<u32> {
    // /proc comm may contain spaces or ')' characters. Fields after its final
    // ')' begin with process state, then parent PID.
    stat.rsplit_once(')')?
        .1
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}
fn is_session_parent(pid: u32) -> bool {
    let mut current = std::process::id();
    // labwc may use a short /bin/sh -c wrapper for -S. Permit only that
    // documented launcher chain, not arbitrary apps descended from the session.
    for _ in 0..4 {
        let Some(stat) = read_bounded(&PathBuf::from(format!("/proc/{current}/stat")), 16_384)
        else {
            return false;
        };
        let Some(parent) = std::str::from_utf8(&stat).ok().and_then(parent_pid) else {
            return false;
        };
        if parent == pid {
            return true;
        }
        if parent <= 1 || parent == current {
            return false;
        }
        let Ok(image) = std::fs::read_link(format!("/proc/{parent}/exe")) else {
            return false;
        };
        if !matches!(
            image.file_name().and_then(|name| name.to_str()),
            Some("sh" | "dash" | "bash" | "busybox")
        ) {
            return false;
        }
        let Some(command) = read_bounded(&PathBuf::from(format!("/proc/{parent}/cmdline")), 16_384)
        else {
            return false;
        };
        if !command
            .split(|byte| *byte == 0)
            .any(|argument| argument.ends_with(b"/windowsphone-session-client"))
        {
            return false;
        }
        current = parent;
    }
    false
}

fn configured_session() -> bool {
    if std::env::var("PHONE_DESKTOP_SESSION").as_deref() != Ok("labwc-v1")
        || std::env::var("DESKTOP_SESSION").as_deref() != Ok("windowsphone")
        || std::env::var("XDG_SESSION_TYPE").as_deref() != Ok("wayland")
        || !std::env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .split(':')
            .any(|desktop| desktop == "WindowsPhone")
    {
        return false;
    }
    let Some(pid) = std::env::var("LABWC_PID")
        .ok()
        .and_then(|pid| pid.parse::<u32>().ok())
        .filter(|pid| *pid > 1)
    else {
        return false;
    };
    if !is_session_parent(pid) {
        return false;
    }
    let Ok(executable) = std::fs::read_link(format!("/proc/{pid}/exe")) else {
        return false;
    };
    if executable.file_name().and_then(|name| name.to_str()) != Some("labwc") {
        return false;
    }
    let Some(command) = read_bounded(&PathBuf::from(format!("/proc/{pid}/cmdline")), 16_384) else {
        return false;
    };
    let args = command.split(|byte| *byte == 0).collect::<Vec<_>>();
    if !args.windows(2).any(|args| {
        matches!(args[0], b"-S" | b"--session")
            && args[1].ends_with(b"/windowsphone-session-client")
    }) {
        return false;
    }
    let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
    else {
        return false;
    };
    let Some(display) = std::env::var_os("WAYLAND_DISPLAY") else {
        return false;
    };
    let display = PathBuf::from(display);
    if display.components().count() != 1 || display.is_absolute() {
        return false;
    }
    let Ok(socket) = std::fs::metadata(runtime.join(display)) else {
        return false;
    };
    let Ok(process) = std::fs::metadata(format!("/proc/{pid}")) else {
        return false;
    };
    socket.file_type().is_socket() && socket.uid() == process.uid()
}

pub(super) fn state(mode: HostMode) -> PlatformState {
    let catalog = gio().is_ok();
    let active = configured_session();
    let available = ["/usr/bin/labwc", "/usr/local/bin/labwc"]
        .iter()
        .any(|path| executable(path));
    let mut features = Vec::new();
    if catalog {
        features.push(HostFeature::InstalledApps);
    }
    if settings().is_some() {
        features.push(HostFeature::SystemSettings);
    }
    if active || available {
        features.push(HostFeature::ShellSession);
    }
    PlatformState {
        platform: phone_host::HostPlatform::Linux,
        mode,
        launcher: HostRole::Unsupported,
        shell: if active {
            HostRole::Active
        } else if available {
            HostRole::Available
        } else {
            HostRole::Unsupported
        },
        features,
        detail: if active {
            "Running as the configured Windows Phone desktop-session client under labwc. Labwc owns Wayland display, window management and app switching; OS services depend on the installed session services.".into()
        } else {
            "Desktop application and launcher modes use the installed GIO catalog. Shell mode requires the opt-in Windows Phone labwc login session; a fullscreen window does not establish a shell session. No default HOME-app role exists on this desktop.".into()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn desktop_ids_cannot_be_paths_or_command_lines() {
        assert!(valid_id("org.example.Editor.desktop"));
        for invalid in [
            "../evil.desktop",
            "/tmp/evil.desktop",
            "a.desktop\0",
            "a.desktop --flag",
            ".desktop",
            "a\\b.desktop",
        ] {
            assert!(!valid_id(invalid), "{invalid:?}");
        }
    }
    #[test]
    fn proc_parent_handles_names_with_spaces_and_parentheses() {
        assert_eq!(parent_pid("77 (a tricky ) process) S 42 0 0 0"), Some(42));
        assert_eq!(parent_pid("broken"), None);
    }
    #[test]
    fn settings_requests_use_fixed_program_arguments() {
        assert_eq!(
            settings_command(
                Settings::Gnome("/usr/bin/gnome-control-center"),
                SystemSettingsPage::Network
            ),
            ("/usr/bin/gnome-control-center", &["network"][..])
        );
        assert_eq!(
            settings_command(
                Settings::Kde("/usr/bin/systemsettings"),
                SystemSettingsPage::DefaultApps
            )
            .1,
            &["kcm_componentchooser"]
        );
    }

    #[test]
    fn isolated_gio_catalog_and_activation_protocol() {
        // A subprocess owns its XDG environment and GIO caches. The normal test
        // process never reads or modifies the person's installed-app catalog.
        let root = std::env::temp_dir().join(format!(
            "phone-gio-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        std::fs::create_dir_all(root.join("data/applications")).unwrap();
        std::fs::create_dir_all(root.join("system/applications")).unwrap();
        let _cleanup = Cleanup(root.clone());
        let helper = root.join("fixture app");
        std::fs::write(
            &helper,
            "#!/bin/sh\nprintf '%s' \"$1\" > \"$PHONE_TEST_OUTPUT\"\n",
        )
        .unwrap();
        std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
        let visible = format!("[Desktop Entry]\nType=Application\nName=Fixture Editor\nExec=\"{}\" \"literal; stay an argument\"\nOnlyShowIn=WindowsPhone;\n", helper.display());
        std::fs::write(root.join("data/applications/fixture.desktop"), &visible).unwrap();
        std::fs::write(
            root.join("data/applications/hidden.desktop"),
            format!("{visible}NoDisplay=true\n"),
        )
        .unwrap();
        std::fs::write(
            root.join("data/applications/other.desktop"),
            visible.replace("OnlyShowIn=WindowsPhone;", "OnlyShowIn=AnotherDesktop;"),
        )
        .unwrap();
        std::fs::write(root.join("system/applications/removed.desktop"), &visible).unwrap();
        std::fs::write(
            root.join("data/applications/removed.desktop"),
            "[Desktop Entry]\nType=Application\nHidden=true\n",
        )
        .unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "linux::tests::gio_fixture_child",
                "--ignored",
                "--nocapture",
            ])
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", &root)
            .env("XDG_DATA_HOME", root.join("data"))
            .env("XDG_DATA_DIRS", root.join("system"))
            .env("XDG_CURRENT_DESKTOP", "WindowsPhone")
            .env(
                "DBUS_SESSION_BUS_ADDRESS",
                format!("unix:path={}/no-session-bus", root.display()),
            )
            .env("PHONE_TEST_OUTPUT", root.join("accepted-argument"))
            .env("PHONE_GIO_TEST_FIXTURE", &root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while child.try_wait().unwrap().is_none() {
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let result = child.wait_with_output().unwrap();
        assert!(
            result.status.success(),
            "GIO fixture failed:\n{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        eprintln!("{}", String::from_utf8_lossy(&result.stderr));
    }

    #[test]
    #[ignore = "run only by the isolated parent fixture with its temporary XDG directories"]
    fn gio_fixture_child() {
        let root = PathBuf::from(
            std::env::var_os("PHONE_GIO_TEST_FIXTURE").expect("isolated fixture only"),
        );
        if gio().is_err() {
            // Optional runtime dependency: this is an explicit skip on minimal
            // systems, not evidence that an installed-app catalog was tested.
            eprintln!("GIO runtime unavailable; isolated application protocol was not exercised.");
            return;
        }
        let apps = applications().unwrap();
        assert_eq!(apps.len(), 1, "{apps:?}");
        assert_eq!(apps[0].id, "linux:fixture.desktop");
        assert_eq!(apps[0].title, "Fixture Editor");
        assert!(launch("../../tmp/fixture.desktop").is_err());
        launch(&apps[0].target).unwrap();
        let output = root.join("accepted-argument");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !output.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(
            std::fs::read_to_string(output).unwrap(),
            "literal; stay an argument"
        );
        std::fs::write(
            root.join("data/applications/fixture.desktop"),
            "[Desktop Entry]\nType=Application\nName=Removed\nExec=/bin/true\nHidden=true\n",
        )
        .unwrap();
        assert!(
            launch(&apps[0].target).is_err(),
            "hidden catalog entries must be revalidated"
        );
        eprintln!("GIO fixture: visibility, XDG override, literal argument and stale launch rejection passed.");
    }
}
