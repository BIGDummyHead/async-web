use async_stream::stream;
use futures::{Stream, StreamExt};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

/// # Stream File
/// 
/// Consumes a file path, opens it, turns it into a reader and yiels data to Vec<u8> using the stream! macro.
/// 
/// Turns a file path into a stream.
pub fn stream_file(file_path: String) -> impl Stream<Item = Vec<u8>> {
    stream! {
    let f = File::open(file_path).await;

            if f.is_err() {
                return ;
            }

            let f = f.unwrap();

            //make streamed reader from file
            let mut reader = ReaderStream::new(f);

            while let Some(data) = reader.next().await {

                //no more data to present to client
                if data.is_err() {
                    return;
                }


                let data = data.unwrap();

                //yield data from the file
                yield data.to_vec();
            }


        }
}
