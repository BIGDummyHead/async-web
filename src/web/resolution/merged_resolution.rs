use std::{cell::RefCell, pin::Pin};

use async_stream::stream;
use futures::{Stream, stream::once};
use linked_hash_map::LinkedHashMap;
use tokio_stream::StreamExt;

use crate::web::{Resolution, resolution::empty_content};

//represents a struct that holds the merged struct.
struct MergedResolution {
    headers: RefCell<Option<LinkedHashMap<String, Option<String>>>>,
    stream: RefCell<Option<Pin<Box<dyn Stream<Item = Vec<u8>> + Send>>>>,
}

impl Resolution for MergedResolution {
    fn get_headers(&self) -> LinkedHashMap<String, Option<String>> {
        //borrow the header mutability
        let mut ref_headers = self.headers.borrow_mut();
        //take the headers, none if nothing anyhow
        let taken_headers = ref_headers.take();

        //return the headers only once, if none return none
        if let Some(headers) = taken_headers {
            headers
        } else {
            LinkedHashMap::new()
        }
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
/// Combines the Left and Right resolution into a Merged Resolution.
///
/// It is important to note that the left and right headers become merged (as to avoid resolution conflict).
///
/// The left headers take precedent over the right side headers however.
pub fn and<L, R>(left: L, right: R) -> impl Resolution
where
    L: Resolution,
    R: Resolution,
{
    //get the leftside headers, then the rightside headers
    let left_headers = left.get_headers();
    let mut combined_headers = right.get_headers();

    //place the left hand side on top of the right table
    for (key, value) in left_headers {
        combined_headers.insert(key, value);
    }

    //combine the streams to do one after another, create a new stream that is the merged.
    let mut merged = left.get_content().merge(right.get_content());
    let content_stream = stream! {
        while let Some(content) = merged.next().await {
            yield content;
        }
    };

    MergedResolution {
        headers: RefCell::new(Some(combined_headers)),
        //refcell, some, pin box
        stream: RefCell::new(Some(Box::pin(content_stream))),
    }
}
