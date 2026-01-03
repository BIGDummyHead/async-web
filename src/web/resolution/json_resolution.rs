use std::pin::Pin;

use futures::{Stream, stream};
use serde::Serialize;
use serde_json::{Value, json};

use crate::web::{Resolution, resolution::get_status_header};

/// ## JSON Resolution
/// 
/// Implementation of the Resolution trait. Allows for you send JSON based content back to a client.
/// 
/// The usage may differ from other resolutions you have experience but works just as easily.
/// 
/// 
/// ## Example 
/// 
/// ```
/// //assume that we are in a resolution function for our route.
/// 
/// //a person object exist that just has a name and age
/// let person = Person::new("John Doe", 32);
/// 
/// let mut j_resolution = JsonResolution::new(person);
/// 
/// //we can also change the status of this resolution (it is by default 200)
/// j_resolution.set_status(200);
/// 
/// //boxes the value for you
/// return j_resolution.into_resolution();
/// ```
pub struct JsonResolution {
    json_value: String,
    status_code: i32,
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

        let json_res = Self {
            json_value: serialize_result.unwrap(),
            status_code: 200,
        };

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
    fn get_headers(&self) -> Pin<Box<dyn Future<Output = Vec<String>> + Send + '_>> {
        Box::pin(async move {
            vec![
                get_status_header(self.status_code),
                "Content-Type: application/json".to_string(),
            ]
        })
    }

    fn get_content(&self) ->  Pin<Box<dyn Stream<Item = Vec<u8>> + Send + 'static>>  {
        
        let json_value = self.json_value.clone();
        
        Box::pin(stream::once( async move { 
            json_value.into_bytes() 
        }))
    }
}
