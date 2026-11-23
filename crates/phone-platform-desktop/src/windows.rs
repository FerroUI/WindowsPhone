//! Enumerate the shell's AppsFolder (including desktop and packaged apps), then
//! activate a freshly resolved item through its PIDL. No command interpreter,
//! executable path, or URI supplied by the caller reaches ShellExecuteEx.
use super::{
    application, Application, HostFeature, HostMode, HostRole, PlatformState,
    MAX_DEVICE_APPLICATIONS,
};
use ::windows::{
    core::{w, PCWSTR, PWSTR},
    Win32::{
        Foundation::{CloseHandle, HANDLE, RPC_E_CHANGED_MODE},
        System::{
            Com::{
                CoInitializeEx, CoTaskMemFree, CoUninitialize, IBindCtx, COINIT_APARTMENTTHREADED,
                COINIT_DISABLE_OLE1DDE,
            },
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
                TH32CS_SNAPPROCESS,
            },
            SystemInformation::GetSystemDirectoryW,
            Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
                PROCESS_QUERY_LIMITED_INFORMATION,
            },
        },
        UI::{
            Shell::{
                BHID_EnumItems, FOLDERID_AppsFolder, IEnumShellItems, IShellItem,
                SHGetIDListFromObject, SHGetKnownFolderItem, ShellExecuteExW, KF_FLAG_DEFAULT,
                SEE_MASK_FLAG_NO_UI, SEE_MASK_INVOKEIDLIST, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW,
                SIGDN, SIGDN_DESKTOPABSOLUTEPARSING, SIGDN_NORMALDISPLAY,
            },
            WindowsAndMessaging::SW_SHOWNORMAL,
        },
    },
};
use phone_host::platform::{PlatformResponse, PlatformTask, SystemSettingsPage};
use std::{ffi::c_void, mem::size_of};

struct Apartment(bool);
impl Apartment {
    fn enter() -> Result<Self, String> {
        let result =
            unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
        if result == RPC_E_CHANGED_MODE {
            return Ok(Self(false));
        }
        result.ok().map_err(|error| error.to_string())?;
        Ok(Self(true))
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        if self.0 {
            unsafe {
                CoUninitialize();
            }
        }
    }
}
struct Allocation(*mut c_void);
impl Drop for Allocation {
    fn drop(&mut self) {
        unsafe {
            CoTaskMemFree(Some(self.0));
        }
    }
}
struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

fn display_name(item: &IShellItem, kind: SIGDN) -> Result<String, String> {
    let value = unsafe { item.GetDisplayName(kind) }.map_err(|error| error.to_string())?;
    let _allocation = Allocation(value.0.cast());
    let value = unsafe { value.to_string() }.map_err(|error| error.to_string())?;
    if value.len() > 16_384 {
        return Err("An application identity exceeds its limit.".into());
    }
    Ok(value)
}
fn enumerate() -> Result<IEnumShellItems, String> {
    let folder: IShellItem =
        unsafe { SHGetKnownFolderItem(&FOLDERID_AppsFolder, KF_FLAG_DEFAULT, None) }
            .map_err(|error| error.to_string())?;
    unsafe { folder.BindToHandler(None::<&IBindCtx>, &BHID_EnumItems) }
        .map_err(|error| error.to_string())
}
fn each_item(mut operation: impl FnMut(IShellItem) -> Result<bool, String>) -> Result<(), String> {
    let enumerator = enumerate()?;
    for _ in 0..=MAX_DEVICE_APPLICATIONS {
        let mut item = [None];
        let mut fetched = 0;
        unsafe { enumerator.Next(&mut item, Some(&mut fetched)) }
            .map_err(|error| error.to_string())?;
        if fetched == 0 {
            return Ok(());
        }
        if let Some(item) = item[0].take() {
            if !operation(item)? {
                return Ok(());
            }
        }
    }
    Err("The installed application catalog exceeds its limit.".into())
}

pub(super) fn applications() -> Result<Vec<Application>, String> {
    let _apartment = Apartment::enter()?;
    let mut applications = Vec::new();
    each_item(|item| {
        if let (Ok(identity), Ok(title)) = (
            display_name(&item, SIGDN_DESKTOPABSOLUTEPARSING),
            display_name(&item, SIGDN_NORMALDISPLAY),
        ) {
            if let Some(application) = application("windows", &identity, &title, &identity) {
                applications.push(application);
            }
        }
        Ok(true)
    })?;
    Ok(applications)
}

pub(super) fn launch(identity: &str) -> Result<(), String> {
    let _apartment = Apartment::enter()?;
    let mut resolved = None;
    // The OS re-enumeration ensures a removed app or a stale parsing name does
    // not turn into a general filesystem/URL launch operation.
    each_item(|item| {
        if display_name(&item, SIGDN_DESKTOPABSOLUTEPARSING)
            .ok()
            .as_deref()
            == Some(identity)
        {
            resolved = Some(item);
            return Ok(false);
        }
        Ok(true)
    })?;
    let item = resolved
        .ok_or("That application is no longer installed. Refresh the device applications.")?;
    let pidl = unsafe { SHGetIDListFromObject(&item) }.map_err(|error| error.to_string())?;
    let _pidl = Allocation(pidl.cast());
    let mut request = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_INVOKEIDLIST | SEE_MASK_FLAG_NO_UI | SEE_MASK_NOASYNC,
        lpVerb: w!("open"),
        lpIDList: pidl.cast(),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    unsafe { ShellExecuteExW(&mut request) }
        .map_err(|error| format!("Windows could not activate the application: {error}"))
}

fn settings_uri(page: SystemSettingsPage) -> &'static str {
    match page {
        SystemSettingsPage::Application => "ms-settings:appsfeatures",
        SystemSettingsPage::Network => "ms-settings:network",
        SystemSettingsPage::Notifications => "ms-settings:notifications",
        SystemSettingsPage::DefaultApps => "ms-settings:defaultapps",
        SystemSettingsPage::Wifi => "ms-settings:network-wifi",
        SystemSettingsPage::Bluetooth => "ms-settings:bluetooth",
        SystemSettingsPage::Cellular => "ms-settings:network-cellular",
        SystemSettingsPage::Airplane => "ms-settings:network-airplanemode",
        SystemSettingsPage::Hotspot => "ms-settings:network-mobilehotspot",
        SystemSettingsPage::Vpn => "ms-settings:network-vpn",
        SystemSettingsPage::Nfc => "ms-settings:proximity",
        SystemSettingsPage::Location => "ms-settings:privacy-location",
        SystemSettingsPage::Display | SystemSettingsPage::Brightness => "ms-settings:display",
        SystemSettingsPage::Rotation => "ms-settings:screenrotation",
        SystemSettingsPage::ScreenTimeout => "ms-settings:powersleep",
        SystemSettingsPage::Sound | SystemSettingsPage::AudioOutput => "ms-settings:sound",
        SystemSettingsPage::DoNotDisturb => "ms-settings:quiethours",
        SystemSettingsPage::Battery | SystemSettingsPage::BatterySaver => {
            "ms-settings:batterysaver"
        }
        SystemSettingsPage::Storage => "ms-settings:storagesense",
        SystemSettingsPage::Security => "ms-settings:windowsdefender",
        SystemSettingsPage::Biometrics => "ms-settings:signinoptions",
        SystemSettingsPage::Accounts => "ms-settings:emailandaccounts",
        SystemSettingsPage::Sync => "ms-settings:sync",
        SystemSettingsPage::DateTime => "ms-settings:dateandtime",
        SystemSettingsPage::Language => "ms-settings:regionlanguage",
        SystemSettingsPage::Keyboard => "ms-settings:typing",
        SystemSettingsPage::Accessibility => "ms-settings:easeofaccess",
        SystemSettingsPage::Privacy | SystemSettingsPage::Permissions => "ms-settings:privacy",
        SystemSettingsPage::NotificationAccess => "ms-settings:notifications",
        SystemSettingsPage::WriteSettings => "ms-settings:appsfeatures",
        SystemSettingsPage::Phone | SystemSettingsPage::Messaging => "ms-settings:defaultapps",
        SystemSettingsPage::DataUsage | SystemSettingsPage::DataSaver => "ms-settings:datausage",
        SystemSettingsPage::Cast => "ms-settings:project",
        SystemSettingsPage::NearbyShare => "ms-settings:crossdevice",
        SystemSettingsPage::NightLight => "ms-settings:nightlight",
        SystemSettingsPage::SystemUpdate => "ms-settings:windowsupdate",
        SystemSettingsPage::About => "ms-settings:about",
    }
}
pub(super) fn open_settings(page: SystemSettingsPage) -> PlatformTask {
    Box::pin(async move {
        phone_host::native::background::run_system(move || {
            let _apartment = Apartment::enter()?;
            // This URI is selected by a closed enum, not supplied by an app.
            phone_host::native::open_external(std::ffi::OsStr::new(settings_uri(page)))
        })
        .await?;
        Ok(PlatformResponse::Accepted {
            message: "Windows accepted the Settings-page request.".into(),
        })
    })
}

#[link(name = "slc")]
extern "system" {
    fn SLGetWindowsInformationDWORD(name: PCWSTR, value: *mut u32) -> ::windows::core::HRESULT;
}
fn shell_licensed() -> bool {
    let mut enabled = 0;
    unsafe {
        SLGetWindowsInformationDWORD(w!("EmbeddedFeature-ShellLauncher-Enabled"), &mut enabled)
    }
    .is_ok()
        && enabled != 0
}
fn process_image(pid: u32) -> Option<String> {
    let process =
        Handle(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?);
    let mut path = vec![0u16; 32768];
    let mut count = path.len() as u32;
    unsafe {
        QueryFullProcessImageNameW(
            process.0,
            PROCESS_NAME_WIN32,
            PWSTR(path.as_mut_ptr()),
            &mut count,
        )
    }
    .ok()?;
    Some(String::from_utf16_lossy(&path[..count as usize]))
}
fn configured_shell() -> bool {
    let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
        return false;
    };
    let snapshot = Handle(snapshot);
    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut result = unsafe { Process32FirstW(snapshot.0, &mut entry) };
    let mut parent = None;
    let mut visited = 0;
    while result.is_ok() && visited < 32768 {
        visited += 1;
        if entry.th32ProcessID == std::process::id() {
            parent = Some(entry.th32ParentProcessID);
            break;
        }
        result = unsafe { Process32NextW(snapshot.0, &mut entry) };
    }
    let Some(image) = parent.and_then(process_image) else {
        return false;
    };
    let mut system = vec![0u16; 32768];
    let length = unsafe { GetSystemDirectoryW(Some(&mut system)) } as usize;
    if length == 0 || length >= system.len() {
        return false;
    }
    let system = String::from_utf16_lossy(&system[..length]);
    // Only the direct, system-owned Shell Launcher host establishes this role.
    // A secondary program launched from a custom shell is not itself the shell.
    ["CustomShellHost.exe", "eshell.exe"]
        .iter()
        .any(|name| image.eq_ignore_ascii_case(&format!("{system}\\{name}")))
}

pub(super) fn state(mode: HostMode) -> PlatformState {
    let active = configured_shell();
    let available = shell_licensed();
    let mut features = vec![HostFeature::InstalledApps, HostFeature::SystemSettings];
    if active || available {
        features.push(HostFeature::ShellSession);
    }
    PlatformState {
        platform: phone_host::HostPlatform::Windows,
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
            "Windows Shell Launcher started this process as its configured desktop shell. The OS still owns window management, security and application activation.".into()
        } else {
            "AppsFolder supplies desktop and packaged applications. Shell mode requires an administrator-deployed Shell Launcher profile on an eligible Enterprise, Education or IoT Enterprise edition. Ordinary launcher mode does not replace Explorer.".into()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_pages_are_closed_os_identifiers() {
        assert_eq!(
            settings_uri(SystemSettingsPage::Application),
            "ms-settings:appsfeatures"
        );
        assert_eq!(
            settings_uri(SystemSettingsPage::Network),
            "ms-settings:network"
        );
        assert_eq!(
            settings_uri(SystemSettingsPage::Notifications),
            "ms-settings:notifications"
        );
        assert_eq!(
            settings_uri(SystemSettingsPage::DefaultApps),
            "ms-settings:defaultapps"
        );
    }
}
