//! Binary Office package boundary. Documents are parsed and validated in Rust
//! before the persistent phone document changes.
use phone_domain::Id;
use wasm_bindgen::prelude::*;

fn identifier(value: f64) -> Result<Id, JsValue> {
    if !value.is_finite() || value < 1.0 || value.fract() != 0.0 || value > 9_007_199_254_740_991.0
    {
        return Err(JsValue::from_str("Invalid document identifier."));
    }
    Ok(value as Id)
}

#[wasm_bindgen(js_name = importOffice)]
pub fn import_office(name: &str, bytes: Vec<u8>, folder: f64) -> Result<f64, JsValue> {
    let vm =
        crate::app::model().ok_or_else(|| JsValue::from_str("The phone is still starting."))?;
    let mut document =
        phone_domain::import_office(name, &bytes).map_err(|error| JsValue::from_str(&error))?;
    document.folder = if folder == 0.0 {
        None
    } else {
        Some(identifier(folder)?)
    };
    let id = vm
        .import_document(document)
        .map_err(|error| JsValue::from_str(&error))?;
    Ok(id as f64)
}

#[wasm_bindgen(js_name = exportOffice)]
pub fn export_office(id: f64) -> Result<String, JsValue> {
    let id = identifier(id)?;
    let vm =
        crate::app::model().ok_or_else(|| JsValue::from_str("The phone is still starting."))?;
    let data = vm.data();
    let document = data
        .document(id)
        .ok_or_else(|| JsValue::from_str("This document no longer exists."))?;
    let exported =
        phone_domain::export_office(document).map_err(|error| JsValue::from_str(&error))?;
    serde_json::to_string(&exported).map_err(|error| JsValue::from_str(&error.to_string()))
}
