use tonic::{transport::Server, Request, Response, Status};

// This imports the generated gRPC code.
// The name `nsfw_detector_service` matches the package name in your .proto file.
// `nsfw_detector_server` contains the server traits.
// `NsfwDetector` is the name of our service.
// `NsfwDetectionRequest` and `NsfwDetectionResponse` are our message types.
pub mod nsfw_detector_service {
    tonic::include_proto!("nsfw_detector_service"); // The string must match the package name in .proto
}

use nsfw_detector_service::{
    nsfw_detector_server::{NsfwDetector, NsfwDetectorServer},
    NsfwDetectionRequest, NsfwDetectionResponse,
};

// Define a struct that will implement our service's business logic
#[derive(Debug, Default)]
pub struct MyNsfwDetector {}

// Implement the NsfwDetector trait for our struct
#[tonic::async_trait]
impl NsfwDetector for MyNsfwDetector {
    // Implement the DetectNsfw RPC method
    async fn detect_nsfw(
        &self,
        request: Request<NsfwDetectionRequest>,
    ) -> Result<Response<NsfwDetectionResponse>, Status> {
        println!("Got a request: {:?}", request.get_ref());

        let request_data = request.into_inner();

        // Placeholder logic:
        // If image_data is not empty, classify as "SAFE" (for now)
        // Otherwise, return an error or a default classification.
        let classification = if !request_data.image_data.is_empty() {
            "SAFE (Placeholder)".to_string()
        } else {
            "UNKNOWN (No data)".to_string()
        };

        let reply = NsfwDetectionResponse {
            classification_result: classification,
            confidence_score: 0.99, // Placeholder confidence
        };

        Ok(Response::new(reply)) // Send back the response
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?; // Standard gRPC address (IPv6 loopback)
    let detector_service = MyNsfwDetector::default();

    println!("NsfwDetectorServer listening on {}", addr);

    Server::builder()
        .add_service(NsfwDetectorServer::new(detector_service))
        .serve(addr)
        .await?;

    Ok(())
}
