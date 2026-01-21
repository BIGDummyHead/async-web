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

    /// Custom
    /// 
    /// Allows for you to emit a String based on the error received. See ErrorFormatter for the closure.
    /// 
    /// Example: 
    /// 
    /// ```
    /// let custom_handler = Configured::Custom(Box::new(|e| {
    ///     //--snip--
    ///     String::from("this failed because...")
    /// }));
    /// ```
    /// 
    /// The error handler can now to be reused to configure an output.
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
///
/// You can optionally configure the output of the ErrorResolution in a few different idiomatic ways.
///
/// For example:
///
/// ```
///    let e: Box<dyn std::error::Error>; //pretend we have some error we can move.
///
///     ErrorResolution::from_error_with_config(e, Configured::PlainText);
///     //or
///     ErrorResolution::from_error_with_config(e, Configured::Json);
///     //or
///     ErrorResolution::from_error_with_config(e, Configured::Custom(|e| {
///         //return a String based value with the error message.
///         "Custom error!".to_string()
///     }));
///     
/// ```
pub struct ErrorResolution {
    error: Box<dyn std::error::Error + Send + Sync + 'static>,
    config: Configured,
}

impl ErrorResolution {

     /// # From Error With Config
    /// 
    /// Creates a new ErrorResolution (boxed) based on a generic Type that implements the trait `std::error::Error`, outputs the custom config chosen.
    /// 
    /// See the `Configured` enum for outputs.
    pub fn from_error_with_config<T>(
        error: T,
        config: Configured,
    ) -> Box<dyn Resolution + Send + Sync + 'static>
    where
        T: std::error::Error + Send + Sync + 'static,
    {
        let resolve = ErrorResolution { error: Box::new(error), config };

        Box::new(resolve)
    }

    /// # From Error
    /// 
    /// Creates a new ErrorResolution (boxed) based on a generic Type that implements the trait `std::error::Error`. Outputs PlainText.
    /// 
    /// See `from_boxed_error_with_config` for other outputs.
    pub fn from_error<T>(
        error: T,
    ) -> Box<dyn Resolution + Send + Sync + 'static>
    where 
       T: std::error::Error + Send + Sync + 'static {
        return Self::from_error_with_config(error, Configured::PlainText);
    }

    /// # From Boxed Error
    /// 
    /// Creates a new ErrorResolution (boxed) based on a Box<dyn std::error::Error> with PlainText set as the configuration.
    /// 
    /// See `from_boxed_error_with_config` if you would like to customize the output of this resolution.
    pub fn from_boxed_error(error: Box<dyn std::error::Error + Send + Sync + 'static>) 
    -> Box<dyn Resolution + Send + Sync + 'static> {
        return Self::from_boxed_error_with_config(error, Configured::PlainText);
    }

    /// # From Boxed Error with Config
    /// 
    /// Creates a new ErrorResolution (boxed) based on a Box<dyn std::error::Error> and allows for custom configuration.
    /// 
    /// See the Configured Enum for choices of output.
    pub fn from_boxed_error_with_config(error: Box<dyn std::error::Error + Send + Sync + 'static>, config: Configured) 
    -> Box<dyn Resolution + Send + Sync + 'static> {
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
