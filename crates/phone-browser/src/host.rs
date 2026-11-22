use wasm_bindgen::prelude::*;
#[wasm_bindgen(raw_module="./host.js")]
extern "C" {
    #[wasm_bindgen(catch)] pub fn load()->Result<String,JsValue>;
    #[wasm_bindgen(catch)] pub fn save(contents:&str)->Result<(),JsValue>;
    pub fn now()->f64;
    pub fn width()->f64;
    pub fn height()->f64;
    pub fn request(json:&str);
    #[wasm_bindgen(js_name=preserveRecovery)] pub fn preserve_recovery(contents:&str);
    #[wasm_bindgen(js_name=storageError)] pub fn storage_error(message:&str);
}
