// use tonic::{transport::Server, Request, Response, Status};
//
// // This imports the generated gRPC code.
// // The name `nsfw_detector_service` matches the package name in your .proto file.
// // `nsfw_detector_server` contains the server traits.
// // `NsfwDetector` is the name of our service.
// // `NsfwDetectionRequest` and `NsfwDetectionResponse` are our message types.
// pub mod nsfw_detector_service {
//     tonic::include_proto!("nsfw_detector_service"); // The string must match the package name in .proto
// }
//
// use nsfw_detector_service::{
//     nsfw_detector_server::{NsfwDetector, NsfwDetectorServer},
//     NsfwDetectionRequest, NsfwDetectionResponse,
// };
//
// // Define a struct that will implement our service's business logic
// #[derive(Debug, Default)]
// pub struct MyNsfwDetector {}
//
// // Implement the NsfwDetector trait for our struct
// #[tonic::async_trait]
// impl NsfwDetector for MyNsfwDetector {
//     // Implement the DetectNsfw RPC method
//     async fn detect_nsfw(
//         &self,
//         request: Request<NsfwDetectionRequest>,
//     ) -> Result<Response<NsfwDetectionResponse>, Status> {
//         println!("Got a request: {:?}", request.get_ref());
//
//         let request_data = request.into_inner();
//
//         // Placeholder logic:
//         // If image_data is not empty, classify as "SAFE" (for now)
//         // Otherwise, return an error or a default classification.
//         let classification = if !request_data.image_data.is_empty() {
//             "SAFE (Placeholder)".to_string()
//         } else {
//             "UNKNOWN (No data)".to_string()
//         };
//
//         let reply = NsfwDetectionResponse {
//             classification_result: classification,
//             confidence_score: 0.99, // Placeholder confidence
//         };
//
//         Ok(Response::new(reply)) // Send back the response
//     }
// }
//
// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let addr = "[::1]:50051".parse()?; // Standard gRPC address (IPv6 loopback)
//     let detector_service = MyNsfwDetector::default();
//
//     println!("NsfwDetectorServer listening on {}", addr);
//
//     Server::builder()
//         .add_service(NsfwDetectorServer::new(detector_service))
//         .serve(addr)
//         .await?;
//
//     Ok(())
// }

mod model_config;
mod nsfw_model;
use nsfw_model::{GLOBAL_MODEL, ModelError}; // Use our global model and error type
use image::ImageFormat; // For guessing image format
use std::io::Cursor;    // For reading image bytes
use std::sync::Arc;

use tonic::{transport::Server, Request, Response, Status};
use nsfw_detector_service::{
    nsfw_detection_request::ImageSource, // Import the generated ImageSource oneof
};

// This imports the generated gRPC code from the "nsfw_detector_service" package
// defined in your .proto file.
pub mod nsfw_detector_service {
    tonic::include_proto!("nsfw_detector_service");
    // Expose the generated file_descriptor_set for reflection or other tools
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("nsfw_detector_descriptor");
}

// Use the generated types
use nsfw_detector_service::{
    nsfw_detector_server::{NsfwDetector, NsfwDetectorServer},
    ClassificationLabel, // Our new enum
    DetectionScore,      // Our new message for scores
    NsfwDetectionRequest, NsfwDetectionResponse,
};

#[derive(Debug, Default)]
pub struct MyNsfwDetector {}

#[tonic::async_trait]
impl NsfwDetector for MyNsfwDetector {
    async fn detect_nsfw(
        &self,
        request: Request<NsfwDetectionRequest>,
    ) -> Result<Response<NsfwDetectionResponse>, Status> {
        let request_inner = request.into_inner(); // Get the NsfwDetectionRequest
        println!("Got a request: {:?}", request_inner);

        let request_id = request_inner.request_id.clone(); // Store request_id to echo back
        let mut error_message = String::new();
        let mut model_version_str = "unknown".to_string();

        // 1. Get a reference to the global model
        // This will initialize it on first use.
        let model_result = &*GLOBAL_MODEL; // Dereference Lazy to get the Result
        let model_instance: Arc<nsfw_model::NsfwModel> = match model_result {
            Ok(model_arc) => model_arc.clone(),
            Err(model_err) => {
                eprintln!("Failed to load NSFW model: {:?}", model_err);
                let reply = NsfwDetectionResponse {
                    request_id,
                    overall_classification: ClassificationLabel::Unknown as i32,
                    scores: vec![],
                    model_version: model_version_str,
                    error_message: format!("Model loading failed: {}", model_err),
                };
                return Ok(Response::new(reply));
            }
        };

        // 2. Obtain image bytes
        let image_bytes: Vec<u8> = match request_inner.image_source {
            Some(ImageSource::ImageData(data)) => {
                if data.is_empty() {
                    error_message = "Received empty image_data.".to_string();
                    Vec::new()
                } else {
                    data
                }
            }
            Some(ImageSource::ImageUrl(url_str)) => {
                if url_str.is_empty() {
                    error_message = "Received empty image_url.".to_string();
                    Vec::new()
                } else {
                    println!("Fetching image from URL: {}", url_str);
                    // Asynchronously fetch the image
                    // Make sure reqwest is built with an async-compatible TLS backend (like rustls)
                    match reqwest::get(&url_str).await {
                        Ok(response) => match response.bytes().await {
                            Ok(bytes) => bytes.to_vec(),
                            Err(e) => {
                                error_message = format!("Failed to read bytes from URL {}: {}", url_str, e);
                                Vec::new()
                            }
                        },
                        Err(e) => {
                            error_message = format!("Failed to fetch image from URL {}: {}", url_str, e);
                            Vec::new()
                        }
                    }
                }
            }
            None => {
                error_message = "No image_source provided in the request.".to_string();
                Vec::new()
            }
        };
        if !error_message.is_empty() || image_bytes.is_empty() {
            let final_error_message = if error_message.is_empty() {
                "No image data could be processed.".to_string()
            } else {
                error_message
            };
            eprintln!("{}", final_error_message);
            let reply = NsfwDetectionResponse {
                request_id,
                overall_classification: ClassificationLabel::Unknown as i32,
                scores: vec![],
                model_version: model_version_str,
                error_message: final_error_message,
            };
            return Ok(Response::new(reply));
        }

        // 3. Decode image (using tokio::task::spawn_blocking for potentially CPU-bound image decoding)
        let dynamic_image_result = tokio::task::spawn_blocking(move || {
            image::load_from_memory(&image_bytes)
        }).await.map_err(|e| Status::internal(format!("Task join error: {}", e)))?.map_err(|e| {
            error_message = format!("Failed to decode image: {}", e);
            Status::invalid_argument(error_message.clone()) // Use the captured error_message
        });

        let dynamic_image = match dynamic_image_result {
            Ok(img) => img,
            Err(status) => {
                eprintln!("Image decoding failed: {}", status.message());
                let reply = NsfwDetectionResponse {
                    request_id,
                    overall_classification: ClassificationLabel::Unknown as i32,
                    scores: vec![],
                    model_version: model_version_str, // Use already fetched model_version if available
                    error_message: status.message().to_string(),
                };
                return Ok(Response::new(reply));
            }
        };

        // 4. Perform prediction (also in spawn_blocking if model inference is blocking)
        // The `tract` operations themselves can be CPU intensive.
        let prediction_result = tokio::task::spawn_blocking(move || {
            // The model instance is an Arc, so it can be moved into the closure
            model_instance.predict(dynamic_image)
        }).await;

        match prediction_result {
            Ok(Ok((probabilities, version))) => {
                model_version_str = version;
                // Assuming id2label: {"0": "normal", "1": "nsfw"}
                // And probabilities are [prob_normal, prob_nsfw]
                let prob_normal = probabilities.get(0).copied().unwrap_or(0.0);
                let prob_nsfw = probabilities.get(1).copied().unwrap_or(0.0);

                let classification = if prob_nsfw > prob_normal && prob_nsfw > 0.5 { // Example threshold
                    ClassificationLabel::Nsfw
                } else {
                    ClassificationLabel::Normal
                };

                let scores = vec![
                    DetectionScore {
                        label: ClassificationLabel::Normal as i32,
                        score: prob_normal,
                    },
                    DetectionScore {
                        label: ClassificationLabel::Nsfw as i32,
                        score: prob_nsfw,
                    },
                ];

                let reply = NsfwDetectionResponse {
                    request_id,
                    overall_classification: classification as i32,
                    scores,
                    model_version: model_version_str,
                    error_message: "".to_string(),
                };
                Ok(Response::new(reply))
            }
            Ok(Err(model_err)) => {
                let err_msg = format!("Model prediction error: {:?}", model_err);
                eprintln!("{}", err_msg);
                let reply = NsfwDetectionResponse {
                    request_id,
                    overall_classification: ClassificationLabel::Unknown as i32,
                    scores: vec![],
                    model_version: model_version_str,
                    error_message: err_msg,
                };
                Ok(Response::new(reply))
            }
            Err(join_err) => { // Tokio task join error
                let err_msg = format!("Prediction task failed: {}", join_err);
                eprintln!("{}", err_msg);
                let reply = NsfwDetectionResponse {
                    request_id,
                    overall_classification: ClassificationLabel::Unknown as i32,
                    scores: vec![],
                    model_version: model_version_str,
                    error_message: err_msg,
                };
                Ok(Response::new(reply))
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup logging (optional, but good for seeing errors from reqwest, tonic, etc.)
    // You can use `tracing_subscriber` or `env_logger`.
    // Example with env_logger:
    // env_logger::init();
    // Or with tracing:
    tracing_subscriber::fmt::init();
    let addr = "[::1]:50051".parse()?;
    let detector_service = MyNsfwDetector::default();

    println!("NsfwDetectorServer listening on {}", addr);

    // Adding reflection service (optional, but useful for tools like grpcurl)
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(nsfw_detector_service::FILE_DESCRIPTOR_SET)
        .build_v1()?;

    Server::builder()
        .add_service(NsfwDetectorServer::new(detector_service))
        .add_service(reflection_service) // Register the reflection service
        .serve(addr)
        .await?;

    Ok(())
}