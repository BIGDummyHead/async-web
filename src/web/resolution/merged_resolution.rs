use std::{cell::RefCell, pin::Pin};

use async_stream::stream;
use futures::{Stream, stream::once};
use tokio_stream::StreamExt;

use crate::web::{Resolution, resolution::empty_content};

//represents a struct that holds the merged struct.
struct MergedResolution {
    headers: Vec<String>,
    stream: RefCell<Option<Pin<Box<dyn Stream<Item = Vec<u8>> + Send>>>>,
}

impl Resolution for MergedResolution {
    fn get_headers(&self) -> Vec<String> {
        self.headers.clone()
    }

    fn get_content(&self) -> Pin<Box<dyn Stream<Item = Vec<u8>> + Send>> {
        let stream = {
            //borrow this as mut, takes ONCE.
            let mut opt_stream = self.stream.borrow_mut();
            let s = opt_stream.take();

            //no content left to serve, this should never serve content again.
            s.unwrap_or_else(|| Box::pin(once(async move { empty_content() })))
        };

        stream
    }

    fn resolve(self) -> Box<dyn Resolution + Send + 'static> {
        Box::new(self)
    }
}

/// # and
/// 
/// Merges the Left and Right Resolution to create a distinct resolution.
/// 
/// `Note: The right side headers are discarded and the left are preserved.`
/// 
/// The reason for the discard of the right hand side headers are to preserve non-conflicting headers.
/// 
/// ## Returns 
/// 
/// A merged stream as a resolution.
pub fn and<L, R>(left: L, right: R) -> impl Resolution
where
    L: Resolution,
    R: Resolution,
{
    //grab the resolutions headers (left and right)
    let left_headers = left.get_headers();

    //combine the streams to do one after another
    let mut merged = left.get_content().merge(right.get_content());

    let content_stream = stream! {
        while let Some(content) = merged.next().await {
            yield content;
        }
    };

    let content_stream_pin = Box::pin(content_stream);

    MergedResolution {
        headers: left_headers,
        stream: RefCell::new(Some(content_stream_pin)),
    }
}