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

use tonic::{transport::Server, Request, Response, Status};

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

        // --- Placeholder Logic ---
        // In a real implementation, you would:
        // 1. Get image data (from request_inner.image_source: image_data or image_url)
        // 2. Preprocess the image
        // 3. Run inference with the Falconsai model
        // 4. Populate the response based on model output

        let (classification, scores) =
            if request_inner.image_source.is_some() {
                // Simulate some processing if image_source is present
                // For example, if image_data is not empty or image_url is provided
                let is_potentially_valid_input = match &request_inner.image_source {
                    Some(source) => match source {
                        nsfw_detector_service::nsfw_detection_request::ImageSource::ImageData(data) => !data.is_empty(),
                        nsfw_detector_service::nsfw_detection_request::ImageSource::ImageUrl(url) => !url.is_empty(),
                    },
                    None => false,
                };

                if is_potentially_valid_input {
                    // Placeholder: Simulate a "NORMAL" classification
                    (
                        ClassificationLabel::Normal,
                        vec![
                            DetectionScore {
                                label: ClassificationLabel::Normal as i32, // Cast enum to i32 for proto
                                score: 0.95,
                            },
                            DetectionScore {
                                label: ClassificationLabel::Nsfw as i32,
                                score: 0.05,
                            },
                        ],
                    )
                } else {
                    // No valid image source provided
                    (
                        ClassificationLabel::Unknown,
                        vec![
                            DetectionScore {
                                label: ClassificationLabel::Unknown as i32,
                                score: 1.0,
                            },
                        ]
                    )
                }
            } else {
                // No image source provided
                (
                    ClassificationLabel::Unknown,
                    vec![
                        DetectionScore {
                            label: ClassificationLabel::Unknown as i32,
                            score: 1.0,
                        },
                    ]
                )
            };
        let reply = NsfwDetectionResponse {
            request_id, // Echo back the request_id
            overall_classification: classification as i32, // Cast enum to i32 for proto
            scores,
            model_version: "v0.0.1-placeholder".to_string(),
            error_message: if classification == ClassificationLabel::Unknown {
                "No valid image data provided".to_string()
            } else {
                "".to_string()
            },
        };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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