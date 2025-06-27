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
