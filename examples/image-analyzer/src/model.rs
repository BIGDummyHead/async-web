use std::{error::Error, io::Cursor, path::PathBuf};

use candle_core::{Device, Tensor};
use candle_transformers::models::{blip, quantized_blip};
use hf_hub::api::tokio::Api;
use tokenizers::Tokenizer;

pub enum Model {
    M(blip::BlipForConditionalGeneration),
    Q(quantized_blip::BlipForConditionalGeneration),
}

impl Model {
    pub fn text_decoder_forward(
        &mut self,
        xs: &Tensor,
        img_xs: &Tensor,
    ) -> Result<Tensor, Box<dyn std::error::Error>> {
        let decoder = match self {
            Self::M(m) => m.text_decoder().forward(xs, img_xs),
            Self::Q(m) => m.text_decoder().forward(xs, img_xs),
        };

        Ok(decoder?)
    }
}

pub async fn load_image_from_data(
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
pub async fn load_model_file(
    model_id: &str,
    filename: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let api = Api::new()?;

    let api = api.model(model_id.to_string());

    let tokenzier = api.get(filename).await?;

    Ok(tokenzier)
}

pub async fn get_tokenzier() -> Result<Tokenizer, Box<dyn std::error::Error>> {
    let api = Api::new()?;

    let model_id = "Salesforce/blip-image-captioning-large".to_string();
    let api = api.model(model_id);

    let tokenzier = api.get("tokenizer.json").await?;

    Ok(Tokenizer::from_file(tokenzier).map_err(|e| -> Box<dyn std::error::Error> { e })?)
}
