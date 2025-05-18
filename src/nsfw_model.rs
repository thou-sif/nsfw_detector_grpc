use crate::model_config::PreprocessorConfig;
use image::{DynamicImage, Pixel, RgbImage};
use ndarray::{Array, IxDyn}; 
use once_cell::sync::Lazy;
use ort::inputs;

use ort::session::{Session, SessionOutputs};
use ort::session::builder::SessionBuilder;
use ort::error::Error as OrtError;
use std::path::Path;
use std::sync::Arc; 

#[derive(thiserror::Error, Debug)]
pub enum ModelError {
    #[error("ONNX Runtime error: {0}")]
    Ort(#[from] OrtError), 
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
    #[error("Failed to convert model output")]
    OutputConversion,
}


pub struct NsfwModel {
    session: Session, // ort::Session
    preprocessor_config: PreprocessorConfig,
}

impl NsfwModel {
    pub fn new(model_dir: &Path) -> Result<Self, ModelError> {
        let model_path = model_dir.join("model.onnx");
        let preprocessor_config_path = model_dir.join("preprocessor_config.json");

        println!("Loading model from: {:?}", model_path);
        println!("Loading preprocessor config from: {:?}", preprocessor_config_path);


        if !model_path.exists() {
            return Err(ModelError::InvalidPath(format!(
                "Model file not found: {:?}",
                model_path
            )));
        }
        if !preprocessor_config_path.exists() {
            return Err(ModelError::InvalidPath(format!(
                "Preprocessor config file not found: {:?}",
                preprocessor_config_path
            )));
        }

        let preprocessor_config_file = std::fs::File::open(preprocessor_config_path)?;
        let preprocessor_config: PreprocessorConfig = serde_json::from_reader(preprocessor_config_file)?;

        println!("Preprocessor config loaded: {:?}", preprocessor_config);
        println!("Creating model with input image size: {}x{}",
                 preprocessor_config.size.width, preprocessor_config.size.height);

        // 1. Create an ONNX Runtime Environment
        // The environment must be alive for the duration of the session.
        // Using Arc to manage its lifetime alongside the session.
        // let environment = Arc::new(
        //     Environment
        //         .with_name("nsfw_detector_env")
        //         .build()?,
        // );

        // 2. Create a Session
        // SessionBuilder takes a reference to the environment.
        println!("Loading ONNX model with ONNX Runtime...");
        let session = SessionBuilder::new()?
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)? // Optional: for performance
            .with_intra_threads(num_cpus::get() as i16 as usize)? // Optional: use all available CPUs
            .commit_from_file(model_path)?;

        println!("ONNX Runtime session created successfully.");
        // You can print model input/output details if needed:
        // session.inputs.iter().for_each(|input| println!("Input: {:?}", input));
        // session.outputs.iter().for_each(|output| println!("Output: {:?}", output));

        Ok(Self {
            session,
            preprocessor_config,
        })
    }

    fn preprocess(&self, image: DynamicImage) -> Result<Array<f32, IxDyn>, ModelError> {
        let config = &self.preprocessor_config;
        let target_height = config.size.height;
        let target_width = config.size.width;

        // Resize image according to model requirements
        let resized_image = image.resize_exact(
            target_width as u32,
            target_height as u32,
            image::imageops::FilterType::Triangle,
        );
        let rgb_image: RgbImage = resized_image.to_rgb8();

        // Create tensor in NCHW format: [1, 3, Height, Width]
        let mut array = Array::zeros((1, 3, target_height, target_width));

        // Use configuration values for normalization
        let do_normalize = config.do_normalize;
        let do_rescale = config.do_rescale;
        let rescale_factor = config.rescale_factor;
        let image_mean = config.image_mean;
        let image_std = config.image_std;

        // Process each pixel
        for (y, x, pixel) in rgb_image.enumerate_pixels() {
            let rgb = pixel.to_rgb();

            // Apply preprocessing steps according to configuration
            for c in 0..3 {
                let mut pixel_value = rgb[c] as f32;

                // Apply rescaling if configured
                if do_rescale {
                    pixel_value *= rescale_factor;
                } else {
                    // Default rescaling to [0,1] if not using config
                    pixel_value /= 255.0;
                }

                // Apply normalization if configured
                if do_normalize {
                    pixel_value = (pixel_value - image_mean[c]) / image_std[c];
                }

                array[[0, c, y as usize, x as usize]] = pixel_value;
            }
        }

        // Return the dynamic array
        Ok(array.into_dyn())
    }

    // Preprocessing remains largely the same logic, but output is ndarray::Array
    // fn preprocess(&self, image: DynamicImage) -> Result<Array<f32, IxDyn>, ModelError> { // IxDyn for [1,3,H,W]
    //     let config = &self.preprocessor_config;
    //     let target_height = config.size.height;
    //     let target_width = config.size.width;
    // 
    //     let resized_image = image.resize_exact(
    //         target_width as u32,
    //         target_height as u32,
    //         image::imageops::FilterType::Triangle,
    //     );
    //     let rgb_image: RgbImage = resized_image.to_rgb8();
    // 
    //     // Create tensor in NCHW format: [1, 3, Height, Width]
    //     let mut array = Array::zeros((1, 3, target_height, target_width));
    //     for (y, x, pixel) in rgb_image.enumerate_pixels() {
    //         let rgb = pixel.to_rgb(); // Rgb<u8>
    //         // Normalize (pixel / 255.0 - 0.5) / 0.5  which is (pixel / 127.5) - 1.0
    //         array[[0, 0, y as usize, x as usize]] = (rgb[0] as f32 / 127.5) - 1.0; // R
    //         array[[0, 1, y as usize, x as usize]] = (rgb[1] as f32 / 127.5) - 1.0; // G
    //         array[[0, 2, y as usize, x as usize]] = (rgb[2] as f32 / 127.5) - 1.0; // B
    //     }
    //     Ok(array.into_dyn()) // Convert to Array<f32, IxDyn>
    // }
    pub fn predict(&self, image: DynamicImage) -> Result<(Vec<f32>, String), ModelError> {
        // 1. Preprocess the image to get an ndarray
        let processed_tensor: Array<f32, IxDyn> = self.preprocess(image)?; // Shape [1, 3, H, W]

        // 2. The Falconsai model expects input name "pixel_values"
        // ort::inputs! macro can take ndarray directly
        let inputs = inputs!["pixel_values" => processed_tensor.view()]?;

        // 3. Run inference
        let outputs: SessionOutputs = self.session.run(inputs)?;

        // 4. Process the output - use the output name instead of index
        // First, get the output name from the session's output info
        let output_name = self.session.outputs[0].name.clone();
        let output_value = outputs
            .get(&output_name)
            .ok_or(ModelError::OutputFormatUnexpected)?;

        // Try to extract as a float tensor view.
        // The shape will be something like &[1, 2] (batch_size, num_classes)
        let logits_view = output_value.try_extract_tensor::<f32>()?;
        let logits_slice = logits_view.as_slice().ok_or(ModelError::OutputConversion)?;

        let probabilities = softmax(logits_slice);

        
        let model_version = self
            .preprocessor_config
            .image_processor_type 
            .clone();

        Ok((probabilities, model_version))
    }
    
}

fn softmax(data: &[f32]) -> Vec<f32> {
    let max_val = data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let exps: Vec<f32> = data.iter().map(|&x| (x - max_val).exp()).collect();
    let sum_exps: f32 = exps.iter().sum();
    exps.into_iter().map(|x| x / sum_exps).collect()
}

pub static GLOBAL_MODEL: Lazy<Result<Arc<NsfwModel>, ModelError>> = Lazy::new(|| {
    println!("Initializing GLOBAL_MODEL with ONNX Runtime...");
    let model_dir_str = std::env::var("MODEL_DIR").unwrap_or_else(|_| "model".to_string());
    let model_dir = Path::new(&model_dir_str);
    NsfwModel::new(model_dir).map(Arc::new)
});