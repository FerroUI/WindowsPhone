use std::sync::Once;

static FONTS: &[(&str, &[u8])] = &[
    (
        "/Assets/Selawik-Regular.ttf",
        include_bytes!("Assets/Selawik-Regular.ttf"),
    ),
    (
        "/Assets/Selawik-Light.ttf",
        include_bytes!("Assets/Selawik-Light.ttf"),
    ),
    (
        "/Assets/Selawik-Semibold.ttf",
        include_bytes!("Assets/Selawik-Semibold.ttf"),
    ),
    (
        "/Assets/Selawik-Bold.ttf",
        include_bytes!("Assets/Selawik-Bold.ttf"),
    ),
];

pub fn register() {
    static REGISTERED: Once = Once::new();
    REGISTERED.call_once(|| ferroui_base::platform::register_assets(crate::ASSEMBLY, FONTS));
}
