//! Dispatch bounded SDK HTTPS tasks through the separate system worker.
use ferroui_base::threading::Dispatcher;
pub fn start(request_id: u32, url: String, max_bytes: u32) {
    if !phone_ui::is_http_task_pending(request_id) {
        return;
    }
    drop(Dispatcher::current_dispatcher().spawn_local(async move {
        let result =
            crate::background::run_system(move || phone_host::native::http::get(&url, max_bytes))
                .await;
        phone_ui::complete_http(request_id, result);
    }));
}
