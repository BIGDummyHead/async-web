use std::{ fmt::Debug, panic};

use futures::stream;
use linked_hash_map::LinkedHashMap;
use serde::Serialize;

use crate::{web::{Resolution, resolution::get_status_header}};

/// Idiomatic type alias for converting an Error to a string.
pub type ErrorFormatter = dyn Fn(&Box<dyn std::error::Error + Send>) -> String + Send;

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

/// debug impl
impl std::fmt::Debug for Configured {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Configured::PlainText => write!(f, "PlainText"),
            Configured::Json => write!(f, "Json"),
            Configured::Custom(_) => write!(f, "Custom(...)"),
        }
    }
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
#[derive(Debug)]
pub struct ErrorResolution {
    error: Box<dyn std::error::Error + Send + 'static>,
    config: Configured,

    /// The error code
    /// 
    /// Set to 500 initially, you can change this however.
    pub code: i32
}

impl ErrorResolution {
    /// # from_error
    ///
    /// Converts an error into a `ErrorResolution` resolution.
    ///
    /// `Note: If your error is already boxed, this may be inefficient as it double boxes, please see from_boxed`
    ///
    /// ## Example
    ///
    /// ```
    /// //note how we map the error into a resolution
    /// /// assume that E is not boxed in this scenario
    /// let result: Result<CustomResolution, ErrorResolution> =
    ///     get_some_resolution()
    ///     .map_err(|e| {
    ///         ErrorResolution::from_error(e, None)
    ///      });
    /// ```
    pub fn from_error<T>(error: T, config: impl Into<Option<Configured>>) -> Self
    where
        T: std::error::Error + 'static,
    {
        let error = Box::new(error);

        Self::from_boxed(error, config)
    }

    /// # from_boxed
    ///
    /// Converts a boxed error into a `ErrorResolution` resolution.
    ///
    /// `Note: if your error is not boxed, see from_error<T>`
    ///
    /// ## Example
    ///
    /// ```
    /// //note how we map the error into a resolution
    /// //assume E in this scenario is Boxed.
    /// let result: Result<CustomResolution, ErrorResolution> =
    ///     get_some_resolution()
    ///     .map_err(|e| {
    ///         ErrorResolution::from_boxed(e, None)
    ///      });
    /// ```
    pub fn from_boxed(
        error: Box<dyn std::error::Error>,
        config: impl Into<Option<Configured>>,
    ) -> Self {
        Self {
            error: InnerError::new_box(error),
            config: config.into().unwrap_or(Configured::PlainText),
            code: 500
        }
    }
}

impl Resolution for ErrorResolution {
    //outputs 500 header
    fn get_headers(&self) -> LinkedHashMap<String, Option<String>> {
        let mut hmap = LinkedHashMap::new();

        let header = get_status_header(self.code);

        hmap.insert(header.0, Some(header.1));

        hmap
    }

    /// returns an outputted content
    fn get_content(&self) -> std::pin::Pin<Box<dyn futures::Stream<Item = Vec<u8>> + Send>> {
        let error_bytes = match &self.config {
            Configured::Json => {
                let error = CaptureJsonErr {
                    code: self.code,
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

    fn resolve(self) -> Box<dyn Resolution + Send + 'static> {
        Box::new(self)
    }
}

/// # Inner Error
///
/// The inner error works as a container, it holds the Boxed error that is non-thread safe.
///
/// The Inner Error then implments the following traits
///
/// * std::error::Error
/// * std::fmt::Display
/// * core::marker::Send
///
/// For formatting, it relies on the format of the inner boxed error. So it uses the exact output as the error passed through.
///
/// This essentially makes a Box<std::error::Error> thread safe.
///
#[derive(Debug)]
struct InnerError {
    error: Box<dyn std::error::Error>,
}

impl InnerError {
    /// Create a new InnerError from a Boxed error.
    fn new_box(contain: Box<dyn std::error::Error>) -> Box<dyn std::error::Error + Send + 'static> {
        let inner = Self { error: contain };

        Box::new(inner)
    }
}

// impl simple Display based on the error string
impl std::fmt::Display for InnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&(*self.error), f)
    }
}

// error for idiomatic returns
impl std::error::Error for InnerError {}

//impl send for this, for sending between async operations
unsafe impl Send for InnerError {}

/// stores the code and message from the error to be serialized if the config of [`ErrorResolution`] is Json
#[derive(Serialize)]
struct CaptureJsonErr {
    code: i32,
    message: String,
}
