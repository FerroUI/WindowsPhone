use super::Position;
use std::time::{Duration, Instant};
use zbus::{OwnedObjectPath, Proxy};

pub(super) fn locate() -> Result<Position, String> {
    zbus::block_on(async {
        let connection = zbus::connection::Builder::system()
            .method_timeout(Duration::from_secs(10))
            .build()
            .await
            .map_err(|e| e.to_string())?;
        let manager = Proxy::new(
            &connection,
            "org.freedesktop.GeoClue2",
            "/org/freedesktop/GeoClue2/Manager",
            "org.freedesktop.GeoClue2.Manager",
        )
        .await
        .map_err(|e| e.to_string())?;
        let path: OwnedObjectPath = manager
            .call("GetClient", &())
            .await
            .map_err(|e| e.to_string())?;
        let client = zbus::proxy::Builder::<Proxy>::new(&connection)
            .destination("org.freedesktop.GeoClue2")
            .path(path.as_str())
            .interface("org.freedesktop.GeoClue2.Client")
            .cache_properties(zbus::proxy::CacheProperties::No)
            .build()
            .await;
        let client = match client {
            Ok(client) => client,
            Err(error) => {
                let _: zbus::Result<()> = manager.call("DeleteClient", &(path.clone(),)).await;
                return Err(error.to_string());
            }
        };
        let result = async {
            client.set_property("DesktopId", "com.ferroui.WindowsPhone").await.map_err(|e| e.to_string())?;
            client.set_property("RequestedAccuracyLevel", 8u32).await.map_err(|e| e.to_string())?;
            let _: () = client.call("Start", &()).await.map_err(|e| e.to_string())?;
            let deadline = Instant::now() + Duration::from_secs(25);
            loop {
                // Property caching is disabled; every iteration is an actual
                // observed provider value, never a guessed fallback location.
                let location: OwnedObjectPath = client.get_property("Location").await.map_err(|e| e.to_string())?;
                if location.as_str() != "/" {
                    let location = Proxy::new(&connection, "org.freedesktop.GeoClue2", location.as_str(),
                        "org.freedesktop.GeoClue2.Location").await.map_err(|e| e.to_string())?;
                    return Ok::<_, String>(Position {
                        latitude: location.get_property("Latitude").await.map_err(|e| e.to_string())?,
                        longitude: location.get_property("Longitude").await.map_err(|e| e.to_string())?,
                        accuracy_metres: location.get_property("Accuracy").await.map_err(|e| e.to_string())?,
                    });
                }
                if Instant::now() >= deadline { return Err("Location acquisition timed out. Check your desktop location permission and provider.".into()); }
                std::thread::sleep(Duration::from_millis(200));
            }
        }.await;
        let _: zbus::Result<()> = client.call("Stop", &()).await;
        let _: zbus::Result<()> = manager.call("DeleteClient", &(path.clone(),)).await;
        result
    })
}
