use std::{sync::mpsc, thread, time::Duration};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(target_os = "windows", test))]
mod windows;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PowerSnapshot {
    pub battery_percent: Option<u8>,
    pub charging: Option<bool>,
    pub external_power: Option<bool>,
    pub power_saver: Option<bool>,
}

fn read_power() -> PowerSnapshot {
    #[cfg(target_os = "linux")]
    {
        linux::read(std::path::Path::new("/sys/class/power_supply"))
    }
    #[cfg(target_os = "windows")]
    {
        windows::read()
    }
    #[cfg(target_os = "macos")]
    {
        macos::read()
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        PowerSnapshot::default()
    }
}

/// One bounded worker per native app. Reading sysfs/OS APIs never blocks the UI
/// dispatcher; only changed snapshots are published, at most once per 30 s.
pub struct PowerMonitor {
    updates: mpsc::Receiver<PowerSnapshot>,
    stop: mpsc::Sender<()>,
    worker: Option<thread::JoinHandle<()>>,
}

impl PowerMonitor {
    pub fn start() -> std::io::Result<Self> {
        let (publish, updates) = mpsc::sync_channel(1);
        let (stop, stopping) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("phone-power".into())
            .spawn(move || {
                let mut previous = None;
                loop {
                    let snapshot = read_power();
                    if previous != Some(snapshot) && publish.try_send(snapshot).is_ok() {
                        previous = Some(snapshot);
                    }
                    if !matches!(
                        stopping.recv_timeout(Duration::from_secs(30)),
                        Err(mpsc::RecvTimeoutError::Timeout)
                    ) {
                        break;
                    }
                }
            })?;
        Ok(Self {
            updates,
            stop,
            worker: Some(worker),
        })
    }

    pub fn take_latest(&self) -> Option<PowerSnapshot> {
        self.updates.try_iter().last()
    }
}

impl Drop for PowerMonitor {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
