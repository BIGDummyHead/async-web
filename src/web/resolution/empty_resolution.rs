use futures::{Stream, stream};
use linked_hash_map::LinkedHashMap;

use crate::{ web::{
    Resolution,
    resolution::{empty_content, get_status_header},
}};

/// ## Empty Resolution
///
/// Implementation of the Resolution trait.
///
/// Simply creates an empty respond to send to the client with a status code you can set.
pub struct EmptyResolution {
    status_code: i32,
}

impl EmptyResolution {
    /// Create a new boxed Empty Resolution
    pub fn status(code: i32) -> Self {
        Self { status_code: code }
    }
}

impl Resolution for EmptyResolution {
    fn get_headers(&self) -> LinkedHashMap<String, Option<String>> {
        let mut hmap = LinkedHashMap::new();

        let header = get_status_header(self.status_code);

        hmap.insert(header.0, Some(header.1));

        hmap
    }

    fn get_content(&self) -> std::pin::Pin<Box<dyn Stream<Item = Vec<u8>> + Send>> {
        Box::pin(stream::once(async move { empty_content() }))
    }

    fn resolve(self) -> Box<dyn Resolution + Send + 'static> {
        Box::new(self)
    }
}
