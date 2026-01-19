use std::panic;

use futures::stream;
use serde::Serialize;

use crate::web::{Resolution, resolution::get_status_header};

/// Idiomatic type alias for converting an Error to a string.
pub type ErrorFormatter = dyn Fn(&Box<dyn std::error::Error + Send + Sync>) -> String + Send + Sync;

/// # Configured
///
/// Configuration settings for the Error resolutions
///
/// Determinent of the output given to the resolver.
pub enum Configured {
    /// Plain Text
    PlainText,
    /// Output is JSON
    Json,
    /// Custom converter, passes the error to the Custom conversion kit, which returns a String.
    Custom(Box<ErrorFormatter>),
}

/// # Error Resolution
///
/// Allows you to take an error and convert to a resolution.
///
/// For example:
///
/// ```
/// 
///   //snip inside of resolution endpoint.
/// 
///   let user: Result<UserLogin, Box<dyn Resolution>> = 
///     serde_json::from_slice(&guard.body)
///     .map_err(ErrorResolution::from_error);
/// 
///   if let Err(e_resolution) = user {
///     return e_resolution;
///   }   
///    
/// ```
pub struct ErrorResolution {
    error: Box<dyn std::error::Error + Send + Sync + 'static>,
    config: Configured,
}

impl ErrorResolution {
    /// # Error Resolution
    /// Create an error resolution based on an error using a configuration.
    ///
    /// Makes creating error based resolutions significantly easier.
    pub fn from_error(
        error: Box<dyn std::error::Error + Send + Sync + 'static>,
        config: Configured,
    ) -> Box<dyn Resolution> {
        let resolve = ErrorResolution { error, config };

        Box::new(resolve)
    }
}

#[derive(Serialize)]
struct JsonError {
    code: i32,
    message: String,
}

impl Resolution for ErrorResolution {
    //outputs 500 header
    fn get_headers(&self) -> std::pin::Pin<Box<dyn Future<Output = Vec<String>> + Send + '_>> {
        Box::pin(async move { vec![get_status_header(500)] })
    }

    /// returns an outputted content
    fn get_content(&self) -> std::pin::Pin<Box<dyn futures::Stream<Item = Vec<u8>> + Send>> {
        let error_bytes = match &self.config {
            Configured::Json => {
                let error = JsonError {
                    code: 500,
                    message: self.error.to_string(),
                };

                let json = serde_json::to_string(&error)
                    .map_err(|err| panic!("{err}"))
                    .unwrap();

                json
            }
            Configured::PlainText => self.error.to_string(),
            Configured::Custom(func) => {
                let result = func(&self.error);
                result
            }
        }
        .into_bytes();

        Box::pin(stream::once(async move { error_bytes }))
    }
}
