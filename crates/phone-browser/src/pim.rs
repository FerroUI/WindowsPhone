//! Binary vCard/iCalendar bridge. JavaScript never parses personal records.
use phone_domain::{Id, interchange::InterchangeFile};
use wasm_bindgen::prelude::*;

fn model() -> Result<std::rc::Rc<phone_state::PhoneViewModel>, JsValue> {
    crate::app::model().ok_or_else(|| JsValue::from_str("The phone is still starting."))
}
fn ids(json: &str) -> Result<Vec<Id>, JsValue> {
    if json.len() > 16_384 {
        return Err(JsValue::from_str("Choose up to 512 items."));
    }
    let values: Vec<Id> =
        serde_json::from_str(json).map_err(|_| JsValue::from_str("Invalid selection."))?;
    if values
        .iter()
        .any(|id| *id == 0 || *id > 9_007_199_254_740_991)
    {
        return Err(JsValue::from_str("Invalid item identifier."));
    }
    Ok(values)
}
fn error(message: String) -> JsValue {
    JsValue::from_str(&message)
}
fn file(value: Result<InterchangeFile, String>) -> Result<String, JsValue> {
    serde_json::to_string(&value.map_err(error)?).map_err(|e| error(e.to_string()))
}

#[wasm_bindgen(js_name = importVcards)]
pub fn import_vcards(bytes: Vec<u8>) -> Result<u32, JsValue> {
    model()?
        .import_vcards(&bytes)
        .map(|ids| ids.len() as u32)
        .map_err(error)
}
#[wasm_bindgen(js_name = exportVcards)]
pub fn export_vcards(selection: &str) -> Result<String, JsValue> {
    file(model()?.export_vcards(&ids(selection)?))
}
#[wasm_bindgen(js_name = importIcalendar)]
pub fn import_icalendar(bytes: Vec<u8>) -> Result<u32, JsValue> {
    model()?
        .import_icalendar(&bytes)
        .map(|ids| ids.len() as u32)
        .map_err(error)
}
#[wasm_bindgen(js_name = exportIcalendar)]
pub fn export_icalendar(selection: &str) -> Result<String, JsValue> {
    file(model()?.export_icalendar(&ids(selection)?))
}
