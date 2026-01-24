use std::pin::Pin;

use futures::{Stream, stream};
use serde::Serialize;
use serde_json::{Value, json};

use crate::web::{
    Resolution,
    resolution::{error_resolution::ErrorResolution, get_status_header},
};

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

    /// # serialize
    /// 
    /// Serializes the value T into a JsonResolution, or if the result fails a ErrorResolution is passed back in JSON format.
    /// 
    /// For example inside of a route.
    /// 
    /// ```
    /// 
    /// let r = route!(req, {
    ///     
    ///     //assume that person derives [serialize] from serde::json
    ///     let person = Person::new("Test", 20);
    /// 
    ///     let json: Result<JsonResolution, ErrorResolution> = JsonResolution::serialize(person);
    /// 
    ///     
    /// 
    /// });
    ///     
    /// 
    /// 
    /// ```
    pub fn serialize<T>(value: T) -> Result<Self, ErrorResolution>
    where
        T: Serialize,
    {
        serde_json::to_string(&value)
            .map(|json| Self {
                json_value: json,
                status_code: 200,
            })
            .map_err(|e| {
                ErrorResolution::from_error(
                    e,
                    super::error_resolution::Configured::Json,
                )
            })
    }

    /// Set the status code of the resolution.
    pub fn set_status(&mut self, status_code: i32) -> () {
        self.status_code = status_code
    }

    /// Convert string based json value back to a serde::Value
    pub fn convert_to_value(&self) -> Value {
        json!(self.json_value)
    }
}


impl Resolution for JsonResolution {
    
    fn resolve(self) -> Box<dyn Resolution + Send + 'static> {
        Box::new(self)
    }
    
    fn get_headers(&self) -> Pin<Box<dyn Future<Output = Vec<String>> + Send + '_>> {
        Box::pin(async move {
            vec![
                get_status_header(self.status_code),
                "Content-Type: application/json".to_string(),
            ]
        })
    }

    fn get_content(&self) -> Pin<Box<dyn Stream<Item = Vec<u8>> + Send + 'static>> {
        let json_value = self.json_value.clone();

        Box::pin(stream::once(async move { json_value.into_bytes() }))
    }
}


