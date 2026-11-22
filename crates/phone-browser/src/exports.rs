use phone_domain::{AppId,PhoneData,Route};
use wasm_bindgen::prelude::*;
#[wasm_bindgen(js_name=resize)]
pub fn resize(width:f64,height:f64){if let Some(vm)=crate::app::model(){vm.set_viewport(width,height);}}
#[wasm_bindgen(js_name=tick)]
pub fn tick(now:f64){if let Some(vm)=crate::app::model(){vm.set_clock(now as i64);}}
#[wasm_bindgen(js_name=hostStatus)]
pub fn host_status(text:&str){if let Some(vm)=crate::app::model(){vm.set_status(text.into());}}
#[wasm_bindgen(js_name=exportData)]
pub fn export_data()->String {crate::app::model().and_then(|vm|vm.data().to_json().ok()).unwrap_or_default()}
#[wasm_bindgen(js_name=importData)]
pub fn import_data(json:&str)->Result<(),JsValue>{
    let data=PhoneData::from_json(json).map_err(|error|JsValue::from_str(&error.to_string()))?;
    if let Some(vm)=crate::app::model(){vm.replace_data(data).map_err(|e|JsValue::from_str(&e))?;}
    Ok(())
}
#[wasm_bindgen(js_name=currentRoute)]
pub fn current_route()->String {crate::app::model().and_then(|vm|serde_json::to_string(&vm.route()).ok()).unwrap_or_default()}
#[wasm_bindgen(js_name=goBack)]
pub fn go_back(){if let Some(vm)=crate::app::model(){phone_ui::back(&vm);}}
#[wasm_bindgen(js_name=goHome)]
pub fn go_home(){if let Some(vm)=crate::app::model(){vm.home();}}
#[wasm_bindgen(js_name=openApp)]
pub fn open_app(name:&str){if let Some(vm)=crate::app::model(){if let Some(app)=AppId::ALL.iter().find(|app|app.label()==name){vm.navigate(Route::App(*app));}}}
#[wasm_bindgen(js_name=cancelInput)]
pub fn cancel_input(){phone_ui::host_cancel_input();}
#[wasm_bindgen(js_name=imageReady)]
pub fn image_ready(key:String,bytes:Vec<u8>)->Result<(),JsValue>{phone_ui::register_image(key,bytes).map_err(|e|JsValue::from_str(&e))}
#[wasm_bindgen(js_name=photoImported)]
pub fn photo_imported(name:String,source:String){if let Some(vm)=crate::app::model(){vm.add_photo(name,source);}}
#[wasm_bindgen(js_name=trackImported)]
pub fn track_imported(title:String,source:String){if let Some(vm)=crate::app::model(){vm.add_track(title,source);}}
#[wasm_bindgen(js_name=mediaChanged)]
pub fn media_changed(title:String,source:String,playing:bool){phone_ui::show_media(title,source,playing);}
#[wasm_bindgen(js_name=mediaImported)]
pub fn media_imported(title:String,source:String,kind:String){if let Some(vm)=crate::app::model(){vm.add_media(title,source,kind);}}
#[wasm_bindgen(js_name=weatherReady)]
pub fn weather_ready(place:String,temperature:f64,condition:String){phone_ui::set_weather(place,temperature,condition);}
#[wasm_bindgen(js_name=lockPhone)]
pub fn lock_phone(){phone_ui::lock();}
#[wasm_bindgen(js_name=validateData)]
pub fn validate_data(json:&str)->Result<(),JsValue>{PhoneData::from_json(json).map(|_|()).map_err(|e|JsValue::from_str(&e))}
#[wasm_bindgen(js_name=hostState)]
pub fn host_state()->String{crate::app::model().map(|vm|{let data=vm.data();serde_json::json!({"settings":data.settings,"alarms":data.alarms}).to_string()}).unwrap_or_else(||"{}".into())}
