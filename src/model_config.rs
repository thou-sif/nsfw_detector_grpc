// src/model_config.rs
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct PreprocessorConfig {
    pub do_normalize: bool,
    pub do_rescale: bool,
    pub do_resize: bool,
    pub image_mean: [f32; 3],
    pub image_processor_type: String,
    pub image_std: [f32; 3],
    pub resample: u32,
    pub rescale_factor: f32,
    pub size: ImageSize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ImageSize {
    pub height: usize,
    pub width: usize,
}
// You could also add a struct for config.json if you need id2label,
// but for binary NSFW/Normal, we can often infer it.
// For this example, we'll assume "normal" is index 0 and "nsfw" is index 1
// as per Falconsai/nsfw_image_detection/config.json
// "id2label": {
//    "0": "normal",
//    "1": "nsfw"
//  }