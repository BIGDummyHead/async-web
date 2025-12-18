use serde::Serialize;
use serde_json::{Value, json};

use crate::web::{Resolution, resolution::get_status_header};

/// A struct to convert and send json data over an app.
pub struct JsonResolution {
    json_value: String,
    status_code: i32
}

impl JsonResolution {

    /// Serialize and create a new Resolution
    pub fn new<T>(value: T) -> Result<Self, serde_json::Error>
    where
        T: Serialize,
    {
        let serialize_result = serde_json::to_string(&value);

        if let Err(e) = serialize_result {
            return Err(e);
        }

        let json_res = Self { json_value: serialize_result.unwrap(), status_code: 200 };

        Ok(json_res)
    }

    /// Set the status code of the resolution. 
    pub fn set_status(&mut self, status_code: i32) -> () {
        self.status_code = status_code
    }
    
    /// Box and convert to a Resolution safe for async sending.
    pub fn into_resolution(self) -> Box<dyn super::Resolution + Send> {
        let resol = Box::new(self) as Box<dyn super::Resolution + Send>;

        resol
    }

    /// Convert string based json value back to a serde::Value
    pub fn convert_to_value(&self) -> Value {
        json!(self.json_value)
    }
}

impl Resolution for JsonResolution {
    fn get_headers(&self) -> Vec<String> {
        vec![get_status_header(self.status_code), "Content-Type: application/json".to_string()]
    }

    fn get_content(&self) -> String {
        self.json_value.clone()
    }
}
