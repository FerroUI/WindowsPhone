//! Bounded host completions for the shared typed application chooser.
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = isFileTaskPending)]
pub fn is_file_task_pending(request_id: u32) -> bool {
    phone_ui::is_file_task_pending(request_id)
}

#[wasm_bindgen(js_name = completeFileTask)]
pub fn complete_file_task(request_id: u32, name: String, mime: String, bytes: Vec<u8>) -> bool {
    phone_ui::complete_file(
        request_id,
        Ok(Some(phone_ui::PickedFile { name, mime, bytes })),
    )
}

#[wasm_bindgen(js_name = cancelFileTask)]
pub fn cancel_file_task(request_id: u32) -> bool {
    phone_ui::complete_file(request_id, Ok(None))
}

#[wasm_bindgen(js_name = failFileTask)]
pub fn fail_file_task(request_id: u32, error: String) -> bool {
    phone_ui::complete_file(request_id, Err(error.chars().take(1024).collect()))
}
