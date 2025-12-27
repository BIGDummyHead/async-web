use std::io::Cursor;
use std::sync::Arc;
use std::{error::Error, path::PathBuf};

use async_web::web::{App, Method};
use candle_core::{Device, Tensor};
use candle_transformers::models::{blip, quantized_blip};
use hf_hub::api::tokio::Api;
use tokenizers::Tokenizer;
use tokio::task::yield_now;

pub mod alt_text;
pub mod model;
pub mod token_output_stream;

use crate::alt_text::AltText;
use crate::model::Model;
use crate::token_output_stream::TokenOutputStream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let app = create_local_app().await;

    app.start().await.await?;

    Ok(())
}

/// # Create Local App
/// 
/// Binds the app to the local machine to PORT 8080. It then creates a route for `/alt` that takes a file as a body.
async fn create_local_app() -> App {
    let app = App::bind(1000, "127.0.0.1:8080")
        .await
        .expect("App failed to bind to address.");

    app.add_or_panic(
        "/alt",
        Method::POST,
        None,
        Arc::new(|req| {
            Box::pin(async move {
                let request = req.lock().await;

                if request.body.is_empty() {
                    return AltText::with_error("No request body found!".to_string())
                        .as_resolution();
                }

                let file_data = Cursor::new(request.body.clone());

                let alt_text = generate_alt_text(file_data).await;

                if let Err(e) = alt_text {
                    return AltText::with_error(e.to_string()).as_resolution();
                }

                let alt_text = alt_text.unwrap();

                AltText::with_value(alt_text).as_resolution()
            })
        }),
    )
    .await;

    app
}

const SEP_TOKEN_ID: u32 = 102;


async fn load_image_from_data(
    file_data: Cursor<Vec<u8>>,
) -> Result<Tensor, Box<dyn std::error::Error>> {
    let img = image::ImageReader::new(file_data)
        .with_guessed_format()?
        .decode()
        .map_err(|e| Box::new(e) as Box<dyn Error>)?
        .resize_to_fill(384, 384, image::imageops::FilterType::Triangle);

    let img = img.to_rgb8();

    let data = img.into_raw();
    let data = Tensor::from_vec(data, (384, 384, 3), &Device::Cpu)?.permute((2, 0, 1))?;

    let mean =
        Tensor::new(&[0.48145466f32, 0.4578275, 0.40821073], &Device::Cpu)?.reshape((3, 1, 1))?;

    let std = Tensor::new(&[0.26862954f32, 0.261_302_6, 0.275_777_1], &Device::Cpu)?
        .reshape((3, 1, 1))?;

    Ok((data.to_dtype(candle_core::DType::F32)? / 255.)?
        .broadcast_sub(&mean)?
        .broadcast_div(&std)?)
}

/// asynchronously loads the model file quantized
async fn load_model_file(
    model_id: &str,
    filename: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let api = Api::new()?;

    let api = api.model(model_id.to_string());

    let tokenzier = api.get(filename).await?;

    Ok(tokenzier)
}

async fn get_tokenzier() -> Result<Tokenizer, Box<dyn std::error::Error>> {
    let api = Api::new()?;

    let model_id = "Salesforce/blip-image-captioning-large".to_string();
    let api = api.model(model_id);

    let tokenzier = api.get("tokenizer.json").await?;

    Ok(Tokenizer::from_file(tokenzier).map_err(|e| -> Box<dyn std::error::Error> { e })?)
}

/// # Generate Alt Text
/// 
/// Provided raw image bytes, loads the model and tokenizer. 
/// 
/// If the generation of the alt text was successful it will return the prediction.
/// 
async fn generate_alt_text(
    file_data: Cursor<Vec<u8>>,
) -> Result<String, Box<dyn std::error::Error>> {
    let model_id = "lmz/candle-blip";
    let filename = "blip-image-captioning-large-q4k.gguf";

    let model_file = load_model_file(model_id, filename).await?;
    let tokenizer = get_tokenzier().await?;

    let mut tokenizer = TokenOutputStream::new(tokenizer);

    let mut logits_processor =
        candle_transformers::generation::LogitsProcessor::new(1337, None, None);

    let config = blip::Config::image_captioning_large();

    let device = Device::Cpu;

    let image = load_image_from_data(file_data).await?.to_device(&device)?;

    let vb = quantized_blip::VarBuilder::from_gguf(model_file, &device)?;

    let model = quantized_blip::BlipForConditionalGeneration::new(&config, vb)?;

    let image_embeds = image.unsqueeze(0)?.apply(model.vision_model())?;

    let mut model = Model::Q(model);

    let mut predicition = "".to_string();

    let mut token_ids = vec![30522u32];
    for index in 0..1000 {
        let context_size = if index > 0 { 1 } else { token_ids.len() };
        let start_pos = token_ids.len().saturating_sub(context_size);
        let input_ids = Tensor::new(&token_ids[start_pos..], &device)?.unsqueeze(0)?;
        let logits = model.text_decoder_forward(&input_ids, &image_embeds)?;
        let logits = logits.squeeze(0)?;
        let logits = logits.get(logits.dim(0)? - 1)?;
        let token = logits_processor.sample(&logits)?;
        if token == SEP_TOKEN_ID {
            break;
        }
        token_ids.push(token);
        if let Some(t) = tokenizer.next_token(token)? {
            use std::io::Write;
            predicition.push_str(&t);
            std::io::stdout().flush()?;
        }

        yield_now().await;
    }

    if let Some(rest) = tokenizer
        .decode_rest()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?
    {
        predicition.push_str(&rest);
    }
    Ok(predicition)
}
