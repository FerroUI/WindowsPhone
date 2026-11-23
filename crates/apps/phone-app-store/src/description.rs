use phone_domain::AppId;

pub(crate) fn description(app: AppId) -> &'static str {
    match app {
        AppId::Phone => {
            "Keep your call history, find a contact, and dial with your device's phone app."
        }
        AppId::Messaging => {
            "Follow your conversations, write messages, and hand them to your device's SMS composer."
        }
        AppId::People => {
            "Keep names, numbers and email addresses together. Add people and mark your favorites."
        }
        AppId::Outlook => {
            "Read your mail collection and compose messages in your connected email app."
        }
        AppId::Calendar => {
            "See your month at a glance and keep appointments with dates, times and locations."
        }
        AppId::Photos => {
            "Your Camera Roll, albums and favorite pictures. Import, view, zoom, share and delete photos."
        }
        AppId::Camera => {
            "Use your device's camera to capture a moment and add it to your Camera Roll."
        }
        AppId::InternetExplorer => {
            "Open the web from an address bar at the bottom of your screen. Save and manage favorites."
        }
        AppId::Music => "Add songs from your device and enjoy your own music collection.",
        AppId::Video => "Import and play your video files.",
        AppId::Games => "Find your games, resume saved progress, and pin your favorites to Start.",
        AppId::Store => "Find, enable and pin the apps on this phone.",
        AppId::Settings => {
            "Make your phone yours. Pick an accent, change the background, and arrange the Start screen."
        }
        AppId::Alarms => "Create alarms, choose the days they repeat, and turn each one on or off.",
        AppId::Calculator => {
            "A responsive everyday calculator with keyboard support, memory and scientific functions."
        }
        AppId::OneNote => "Capture an idea, write a list, and come back to your notes later.",
        AppId::Maps => "Search OpenStreetMap and explore places around your current location.",
        AppId::Cortana => {
            "Find things on your phone, open apps, call a contact, or continue a search on the web."
        }
        AppId::Weather => "Look up current weather conditions for your location.",
        AppId::Files => {
            "Browse your photos, music and documents, or create and restore a phone backup."
        }
        AppId::Office => {
            "Create and edit Word documents, Excel workbooks and PowerPoint presentations. Organize documents and exchange files with your device."
        }
        AppId::BatterySaver => "Reduce unnecessary background activity in this app.",
        AppId::DataSense => "See how much data your phone collection uses and save a backup.",
    }
}

pub(crate) fn device_feature(capability: &phone_sdk::Capability) -> &'static str {
    use phone_sdk::Capability::*;
    match capability {
        All => "all device features",
        Network => "internet",
        Camera => "camera",
        Location => "location",
        Notifications => "notifications",
        Media => "photos, music and videos",
        Contacts => "contacts",
        Calendar => "calendar",
        Clipboard => "clipboard",
        Sharing => "sharing",
        Phone => "phone calls",
        Messaging => "text messages",
        Email => "email",
        Files => "files",
        Backup => "phone backups",
    }
}
