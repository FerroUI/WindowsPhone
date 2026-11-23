use phone_sdk::session::Session;
pub fn swipe(ui: &Session, count: usize, delta: f64) -> bool {
    if count == 0 {
        return false;
    }
    let current = ui.pivot.get_untracked().min(count - 1);
    ui.pivot.set(if delta < 0.0 {
        (current + 1) % count
    } else {
        (current + count - 1) % count
    });
    true
}
