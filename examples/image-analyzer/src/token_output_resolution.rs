use crate::loaded_model::LoadedModel;
use crate::model::{Model, load_image_from_data};
use async_stream::stream;
use async_web::web::Resolution;
use async_web::web::resolution::get_status_header;
use candle_core::{Device, Tensor};
use linked_hash_map::LinkedHashMap;
use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::Stream;

/// Token Output Resolution
///
/// Serves as a way to take Image data and convert to a stream of tokens that can be served to the user to caption the image.
///
/// Implements the `Resolution` trait from async_web.
pub struct TokenOutputResolution {
    file_data: Cursor<Vec<u8>>,
    loaded_model: Arc<Mutex<LoadedModel>>,
}

impl TokenOutputResolution {
    pub fn stream(file_data: Cursor<Vec<u8>>, loaded_model: Arc<Mutex<LoadedModel>>) -> Self {
        Self {
            file_data,
            loaded_model,
        }
    }
}

impl Resolution for TokenOutputResolution {
    fn get_headers(&self) -> LinkedHashMap<String, Option<String>> {
        let mut hmap = LinkedHashMap::new();

        let (k, v ) = get_status_header(200);
        hmap.insert(k, Some(v));

        hmap
    }

    /// Provided raw image bytes, loads the model and tokenizer.
    ///
    /// If the generation of the alt text was successful it will return the prediction.
    fn get_content(&self) -> std::pin::Pin<Box<dyn Stream<Item = Vec<u8>> + Send>> {
        let loaded_model = self.loaded_model.clone();

        let file_data = self.file_data.clone();

        let content_stream = stream! {

            let device = Device::Cpu;

            let image = load_image_from_data(file_data)
                .await
                .unwrap()
                .to_device(&device)
                .unwrap();

            let model = {
                let guard = loaded_model.lock().await;
                guard.model.clone()
            };

            let image_embeds = image.unsqueeze(0).unwrap().apply(model.vision_model()).unwrap();

            let mut model = Model::Q(model);


            let sep_token_id: u32 = 102;

            let mut token_ids = vec![30522u32];
            for index in 0..1000 {
                let context_size = if index > 0 { 1 } else { token_ids.len() };
                let start_pos = token_ids.len().saturating_sub(context_size);
                let input_ids = Tensor::new(&token_ids[start_pos..], &device).unwrap().unsqueeze(0).unwrap();
                let logits = model.text_decoder_forward(&input_ids, &image_embeds).unwrap();
                let logits = logits.squeeze(0).unwrap();
                let logits = logits.get(logits.dim(0).unwrap() - 1).unwrap();
                let token = loaded_model.lock().await.logits_processor.sample(&logits).unwrap();

                if token == sep_token_id {
                    break;
                }

                token_ids.push(token);
                if let Some(t) = loaded_model.lock().await.tokenizer.next_token(token).unwrap() {
                    yield t.into_bytes();
                }
            }

            if let Some(rest) = loaded_model.lock().await.tokenizer
                .decode_rest()
                .unwrap()
            {
                yield rest.into_bytes();
            }
        };

        Box::pin(content_stream)
    }

    fn resolve(self) -> Box<dyn Resolution + Send + 'static> {
        Box::new(self)
    }
}
