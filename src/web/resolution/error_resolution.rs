use std::{fmt::Debug, panic};

use futures::stream;
use serde::Serialize;

use crate::web::{Resolution, resolution::get_status_header};

/// Idiomatic type alias for converting an Error to a string.
pub type ErrorFormatter = dyn Fn(&Box<dyn std::error::Error + Send >) -> String + Send ;

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
    error: Box<dyn std::error::Error + Send  + 'static>,
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
    ) -> Self
    where
        T: std::error::Error + 'static,
    {
        let error = InnerError::new_box(Box::new(error));

        let resolve = ErrorResolution { error, config };

        resolve
    }

    /// # From Error
    /// 
    /// Creates a new ErrorResolution (boxed) based on a generic Type that implements the trait `std::error::Error`. Outputs PlainText.
    /// 
    /// See `from_boxed_error_with_config` for other outputs.
    pub fn from_error<T>(
        error: T,
    ) -> Self
    where 
       T: std::error::Error + 'static {
        return Self::from_error_with_config(error, Configured::PlainText);
    }

    /// # From Boxed Error
    /// 
    /// Creates a new ErrorResolution (boxed) based on a Box<dyn std::error::Error> with PlainText set as the configuration.
    /// 
    /// See `from_boxed_error_with_config` if you would like to customize the output of this resolution.
    pub fn from_boxed_error(error: Box<dyn std::error::Error>) 
    -> Self {
        return Self::from_boxed_error_with_config(error, Configured::PlainText);
    }

    /// # From Boxed Error with Config
    /// 
    /// Creates a new ErrorResolution (boxed) based on a Box<dyn std::error::Error> and allows for custom configuration.
    /// 
    /// See the Configured Enum for choices of output.
    pub fn from_boxed_error_with_config(error: Box<dyn std::error::Error>, config: Configured) 
    -> Self {

        let error = InnerError::new_box(error);

        let resolve = ErrorResolution { error, config };
        resolve
    }

    
}


// struct that has a code and message, used for the Configured output of Json
#[derive(Serialize)]
struct InternalJsonResultError {
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
                let error = InternalJsonResultError {
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
    error: Box<dyn std::error::Error>
}

impl InnerError {

    /// Create a new InnerError from a Boxed error.
    fn new_box(contain: Box<dyn std::error::Error>) -> Box<dyn std::error::Error + Send + 'static> {
        let inner = Self {
            error: contain
        };

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
unsafe impl Send for InnerError{}