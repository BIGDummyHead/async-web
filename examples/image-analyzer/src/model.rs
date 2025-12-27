use candle_core::Tensor;
use candle_transformers::models::{blip, quantized_blip};

pub enum Model {
    M(blip::BlipForConditionalGeneration),
    Q(quantized_blip::BlipForConditionalGeneration),
}

impl Model {
    pub fn text_decoder_forward(&mut self, xs: &Tensor, img_xs: &Tensor) -> Result<Tensor, Box<dyn std::error::Error>> {
        let decoder = match self {
            Self::M(m) => m.text_decoder().forward(xs, img_xs),
            Self::Q(m) => m.text_decoder().forward(xs, img_xs),
        };

        Ok(decoder?)
    }
}