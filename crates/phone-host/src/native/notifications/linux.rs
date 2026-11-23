use crate::HostPermission;
use std::{collections::HashMap, time::Duration};
use zbus::{Connection, Proxy, Variant};

async fn connection() -> zbus::Result<Connection> {
    zbus::connection::Builder::session()
        .method_timeout(Duration::from_secs(5))
        .build()
        .await
}

async fn proxy(connection: &Connection) -> zbus::Result<Proxy<'_>> {
    Proxy::new(
        connection,
        "org.freedesktop.Notifications",
        "/org/freedesktop/Notifications",
        "org.freedesktop.Notifications",
    )
    .await
}

pub(super) fn permission() -> Result<HostPermission, String> {
    zbus::block_on(async {
        let connection = connection().await.map_err(|e| e.to_string())?;
        let proxy = proxy(&connection).await.map_err(|e| e.to_string())?;
        let _: (String, String, String, String) = proxy
            .call("GetServerInformation", &())
            .await
            .map_err(|e| e.to_string())?;
        Ok(HostPermission::Granted)
    })
}

pub(super) fn show(title: &str, body: &str) -> Result<(), String> {
    zbus::block_on(async {
        let connection = connection().await.map_err(|e| e.to_string())?;
        let proxy = proxy(&connection).await.map_err(|e| e.to_string())?;
        let hints = HashMap::from([
            ("suppress-sound", Variant::Bool(true)),
            ("desktop-entry", Variant::from("com.ferroui.WindowsPhone")),
        ]);
        let text = escape_markup(body);
        let _: u32 = proxy
            .call(
                "Notify",
                &(
                    "Windows Phone",
                    0u32,
                    "phone",
                    title,
                    text,
                    Vec::<String>::new(),
                    hints,
                    5000i32,
                ),
            )
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    })
}

fn escape_markup(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn personal_content_is_plain_text_not_notification_markup() {
        assert_eq!(
            escape_markup("<b>A & B</b>"),
            "&lt;b&gt;A &amp; B&lt;/b&gt;"
        );
    }
}
