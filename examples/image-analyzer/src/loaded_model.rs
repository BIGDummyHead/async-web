use candle_core::Device;
use candle_transformers::{generation::LogitsProcessor, models::{blip, quantized_blip}};

use crate::{model::{get_tokenzier, load_model_file}, token_output_stream::TokenOutputStream};

pub struct LoadedModel {
    pub tokenizer: TokenOutputStream,
    pub logits_processor: LogitsProcessor,
    pub config: blip::Config,
    pub model: quantized_blip::BlipForConditionalGeneration
}

impl LoadedModel {
    pub async fn new() -> Self {
        let model_id = "lmz/candle-blip";
        let filename = "blip-image-captioning-large-q4k.gguf";

        let model_file = load_model_file(model_id, filename).await.unwrap();
        let tokenizer = get_tokenzier().await.unwrap();

        let tokenizer = TokenOutputStream::new(tokenizer);

        let logits_processor =
            candle_transformers::generation::LogitsProcessor::new(1337, None, None);

        let config: blip::Config = blip::Config::image_captioning_large();

        let device = Device::Cpu;
        let vb = quantized_blip::VarBuilder::from_gguf(model_file, &device).unwrap();

        let model: quantized_blip::BlipForConditionalGeneration = quantized_blip::BlipForConditionalGeneration::new(&config, vb).unwrap();

        Self {
            config,
            logits_processor,
            tokenizer,
            model,
        }
    }
}