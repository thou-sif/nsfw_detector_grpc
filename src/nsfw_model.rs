// src/nsfw_model.rs
use crate::model_config::PreprocessorConfig;
use image::{DynamicImage, GenericImageView, Pixel, RgbImage};
use ndarray::{Array, Array4, Axis, IxDyn};
use once_cell::sync::Lazy;
use std::path::Path;
use std::sync::Arc;
use tract_onnx::prelude::*;

// Define a custom error type for model-related operations
#[derive(thiserror::Error, Debug)]
pub enum ModelError {
    #[error("ONNX inference error: {0}")]
    Inference(#[from] tract_onnx::prelude::TractError),
    #[error("Image processing error: {0}")]
    ImageProcessing(#[from] image::ImageError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Configuration error: {0}")]
    Config(#[from] serde_json::Error),
    #[error("Input tensor shape mismatch")]
    InputShapeMismatch,
    #[error("Model output format unexpected")]
    OutputFormatUnexpected,
    #[error("Invalid path for model files: {0}")]
    InvalidPath(String),
}

type TractModel = SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>;

pub struct NsfwModel {
    model: TractModel,
    preprocessor_config: PreprocessorConfig,
    // id2label: HashMap<String, String>, // If you parse config.json
}

impl NsfwModel {
    pub fn new(model_dir: &Path) -> Result<Self, ModelError> {
        let model_path = model_dir.join("model.onnx");
        let preprocessor_config_path = model_dir.join("preprocessor_config.json");

        println!("Loading model from: {:?}", model_path);
        println!("Loading preprocessor config from: {:?}", preprocessor_config_path);

        if !model_path.exists() {
            return Err(ModelError::InvalidPath(format!("Model file not found: {:?}", model_path)));
        }
        if !preprocessor_config_path.exists() {
            return Err(ModelError::InvalidPath(format!("Preprocessor config file not found: {:?}", preprocessor_config_path)));
        }

        let preprocessor_config_file = std::fs::File::open(preprocessor_config_path)?;
        let preprocessor_config: PreprocessorConfig = serde_json::from_reader(preprocessor_config_file)?;

        println!("Preprocessor config loaded: {:?}", preprocessor_config);
        println!("Creating model with input shape: [1, 3, {}, {}]",
                 preprocessor_config.size.height, preprocessor_config.size.width);

        println!("Loading ONNX model...");

        let mut model = tract_onnx::onnx().model_for_path(&model_path)?;

        println!("Converting model to tract format...");
        model = model.with_input_fact(
            0,
            // dt = f32, shape = [1, 3, height, width]
            InferenceFact::dt_shape(
                f32::datum_type(),
                tvec!(
                1,
                3,
                preprocessor_config.size.height as usize,
                preprocessor_config.size.width as usize
            ),
            ),
        )?;
        // optimize + make runnable
        println!("Optimizing model...");
        let model = model.into_optimized()?;
        let model = model.into_runnable()?;

        println!("Model loaded successfully");

        Ok(Self {
            model,
            preprocessor_config,
        })
    }
    

    fn preprocess(&self, image: DynamicImage) -> Result<Tensor, ModelError> {
        let config = &self.preprocessor_config;
        let target_height = config.size.height;
        let target_width = config.size.width;

        // 1. Resize if needed
        let resized_image = if config.do_resize {
            image.resize_exact(
                target_width as u32,
                target_height as u32,
                image::imageops::FilterType::Triangle, // Common filter for resizing
            )
        } else {
            image
        };

        // 2. Convert to RGB8
        let rgb_image: RgbImage = resized_image.to_rgb8();

        // 3. Create tensor with proper normalization
        let array: Array4<f32> = Array4::from_shape_fn((1, 3, target_height, target_width), |(_, c, y, x)| {
            let pixel = rgb_image.get_pixel(x as u32, y as u32);
            let pixel_value = pixel[c] as f32;

            // Apply rescaling if configured
            let rescaled = if config.do_rescale {
                pixel_value * config.rescale_factor
            } else {
                pixel_value / 255.0  // Default normalization to 0-1 range
            };

            // Apply normalization if configured
            if config.do_normalize {
                (rescaled - config.image_mean[c]) / config.image_std[c]
            } else {
                rescaled
            }
        });

        // 4. Convert to tract::Tensor
        Ok(array.into_dyn().into())
    }

    // fn preprocess(&self, image: DynamicImage) -> Result<Tensor, ModelError> {
    //     let config = &self.preprocessor_config;
    //     let target_height = config.size.height;
    //     let target_width = config.size.width;
    //
    //     // 1. Resize
    //     let resized_image = image.resize_exact(
    //         target_width as u32,
    //         target_height as u32,
    //         image::imageops::FilterType::Triangle, // Common filter for resizing
    //     );
    //
    //     // 2. Convert to RGB8 (tract usually expects NCHW: Batch, Channels, Height, Width)
    //     let rgb_image: RgbImage = resized_image.to_rgb8();
    //
    //     // 3. Normalize and create tensor
    //     // The Falconsai ViT model expects normalization with mean [0.5, 0.5, 0.5] and std [0.5, 0.5, 0.5]
    //     // This means pixel values (0-255) are transformed to [-1.0, 1.0]
    //     // Formula: (pixel_value / 255.0 - mean) / std
    //     // For mean=0.5, std=0.5: (pixel_value / 255.0 - 0.5) / 0.5 = (pixel_value / 255.0) * 2.0 - 1.0
    //
    //     // let mut array = Array::zeros((1, target_height, target_width, 3)); // NHWC temporary
    //     // for (y, x, pixel) in rgb_image.enumerate_pixels() {
    //     //     let rgb = pixel.to_rgb();
    //     //     array[(0, y as usize, x as usize, 0)] = (rgb[0] as f32 / 255.0) * 2.0 - 1.0;
    //     //     array[(0, y as usize, x as usize, 1)] = (rgb[1] as f32 / 255.0) * 2.0 - 1.0;
    //     //     array[(0, y as usize, x as usize, 2)] = (rgb[2] as f32 / 255.0) * 2.0 - 1.0;
    //     // }
    //     // // Convert NHWC to NCHW
    //     // let tensor = array.permuted_axes([0, 3, 1, 2]).as_standard_layout().into_dyn();
    //     // Ok(tensor.into()) // Convert to tract::Tensor
    //     if rgb_image.width() != target_width as u32 || rgb_image.height() != target_height as u32 {
    //         return Err(ModelError::InputShapeMismatch);
    //     }
    //     // 3. Normalize and create tensor
    //     let array: Array4<f32> = Array4::from_shape_fn((1, 3, target_height, target_width), |(_, c, y, x)| {
    //         let pixel = rgb_image.get_pixel(x as u32, y as u32);
    //         (pixel[c] as f32 / 127.5) - 1.0
    //     });
    //
    //     // 4. Convert to tract::Tensor
    //     Ok(array.into_dyn().into())
    // }

    pub fn predict(&self, image: DynamicImage) -> Result<(Vec<f32>, String), ModelError> {
        let tensor = self.preprocess(image)?;

        // Run inference
        let result = self.model.run(tvec!(tensor.into()))?; // Pass tensor as TVec

        // The output is usually a 2D tensor (logits) of shape [batch_size, num_classes]
        // For batch size 1, it's [1, 2] for (normal, nsfw)
        let output_tensor = result[0].to_array_view::<f32>()?;
        let probabilities = softmax(output_tensor.as_slice().ok_or(ModelError::OutputFormatUnexpected)?);

        // Assuming id2label: {"0": "normal", "1": "nsfw"}
        // let model_version = self.model.model().meta.get("model_version").unwrap_or(&"unknown".to_string()).clone();

        let model_version = "0.1.0".to_string(); // Replace with actual model version


        Ok((probabilities, model_version))
    }
}

// Helper function for softmax
fn softmax(data: &[f32]) -> Vec<f32> {
    let max_val = data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let exps: Vec<f32> = data.iter().map(|&x| (x - max_val).exp()).collect();
    let sum_exps: f32 = exps.iter().sum();
    exps.into_iter().map(|x| x / sum_exps).collect()
}

// Create a global, lazily initialized instance of the model.
// This loads the model only once when it's first accessed.
pub static GLOBAL_MODEL: Lazy<Result<Arc<NsfwModel>, ModelError>> = Lazy::new(|| {
    let model_dir_str = std::env::var("MODEL_DIR").unwrap_or_else(|_| "model".to_string());
    let model_dir = Path::new(&model_dir_str);
    NsfwModel::new(model_dir).map(Arc::new)
});