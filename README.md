# NSFW Detector gRPC Service

[![Rust](https://img.shields.io/badge/rust-1.78+-orange.svg)](https://www.rust-lang.org/)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

[//]: # ([![CI]&#40;https://github.com/{{YOUR_GITHUB_USERNAME}}/{{YOUR_REPO_NAME}}/actions/workflows/ci.yml/badge.svg&#41;]&#40;https://github.com/{{YOUR_GITHUB_USERNAME}}/{{YOUR_REPO_NAME}}/actions&#41;)

A high-performance, asynchronous gRPC service for NSFW image detection, written in Rust. It leverages the ONNX Runtime for efficient, multi-threaded model inference.

This service is designed to be a fast, reliable, and easy-to-deploy component for content moderation pipelines.

## ✨ Features

-   **High Performance:** Built with Rust, Tokio, and Tonic for asynchronous, non-blocking I/O.
-   **Efficient Inference:** Uses the `ort` crate for direct, multi-threaded access to the ONNX Runtime, maximizing CPU usage for model prediction.
-   **Robust gRPC API:** A well-defined Protobuf API that accepts images via raw bytes or a public URL.
-   **Production Ready:**
    -   Includes a gRPC Reflection Service for easy API discovery with tools like `grpcurl` and Postman.
    -   Configurable via environment variables.
    -   Graceful error handling and informative responses.
-   **Efficient Model Loading:** The ONNX model is loaded only once on startup and shared safely across all requests.

## 🧠 Model

This service uses the `Falconsai/nsfw_image_detection` model, converted to the [ONNX format](https://huggingface.co/onnx-community/nsfw_image_detection-ONNX) for cross-platform compatibility and performance. The model is a binary classifier that outputs scores for `Normal` and `NSFW` categories.

## 🚀 Getting Started

### Prerequisites

1.  **Rust Toolchain:** Install Rust via [rustup](https://rustup.rs/). This project is built with the 2021 edition.
2.  **Protocol Buffers Compiler (`protoc`):** This is required by `tonic-build` to compile the `.proto` file. Follow the [official installation instructions](https://grpc.io/docs/protoc-installation/).

### 1. Clone the Repository

```bash
git clone https://github.com/thou-sif/nsfw_detector_grpc.git
cd nsfw_detector_grpc
```

### 2. Set Up the Model Files

The ONNX model file (`model.onnx`) is approximately 300MB and is **not included** in this Git repository. You must download it before running the service.

We provide a simple shell script to download and place all required files correctly.

**Automated Setup (Recommended):**

```bash
# Make the script executable
chmod +x download_model.sh

# Run the script
./download_model.sh
```

This will create a `model/` directory, download the ONNX model, and copy the `preprocessor_config.json` into it. Your final directory structure should be:

```
model/
├── model.onnx
└── preprocessor_config.json
```

### 3. Build the Service

Build the project in release mode for optimal performance.

```bash
cargo build --release
```

### 4. Run the Server

```bash
cargo run --release
```

By default, the server will start and listen on `[::1]:50051`. You should see output confirming this:

```
NsfwDetectorServer listening on [::1]:50051
```

## ⚙️ Configuration

The server can be configured using environment variables:

| Variable      | Description                                | Default         |
|---------------|--------------------------------------------|-----------------|
| `MODEL_DIR`   | Path to the directory containing model files. | `model`         |
| `SERVER_ADDR` | The address and port for the gRPC server.  | `[::1]:50051`   |

**Example:**
```bash
export SERVER_ADDR="0.0.0.0:8080"
export MODEL_DIR="/opt/models/nsfw"
cargo run --release
```

## 📡 API Usage

You can interact with the running service using a gRPC client like `grpcurl`. The service name is `nsfw_detector_service.NsfwDetector` and the method is `DetectNsfw`.

### Example 1: Detect from a local image file

```bash
# Encode the image to base64 and pass it in the JSON payload
grpcurl -plaintext \
  -d '{"image_data": "'$(base64 -w0 /path/to/your/image.jpg)'", "request_id": "test-local-123"}' \
  [::1]:50051 nsfw_detector_service.NsfwDetector/DetectNsfw
```

### Example 2: Detect from a URL

```bash
# Provide a public URL for the image
grpcurl -plaintext \
  -d '{"image_url": "https://i.imgur.com/8sC24bE.jpeg", "request_id": "test-url-456"}' \
  [::1]:50051 nsfw_detector_service.NsfwDetector/DetectNsfw
```

### Example Response

A successful response will look like this:

```json
{
  "request_id": "test-local-123",
  "overall_classification": "NORMAL",
  "scores": [
    {
      "label": "NORMAL",
      "score": 0.9985416
    },
    {
      "label": "NSFW",
      "score": 0.0014584218
    }
  ],
  "model_version": "Falconsai/nsfw_image_detection",
  "error_message": ""
}
```

## 🏗️ Project Structure

-   `Cargo.toml`: Project dependencies and metadata.
-   `build.rs`: Compiles the `.proto` file during the build process.
-   `proto/nsfw_detector.proto`: The gRPC service and message definitions.
-   `src/main.rs`: The main gRPC server implementation, request handling, and startup logic.
-   `src/nsfw_model.rs`: Handles loading the ONNX model, preprocessing images, and running predictions.
-   `src/model_config.rs`: Structs for deserializing the `preprocessor_config.json`.
-   `download_model.sh`: Helper script to download required model artifacts.

## ❤️ Contributing

Contributions are welcome! If you'd like to help, please feel free to submit a pull request.

1.  Fork the repository.
2.  Create a new branch (`git checkout -b feature/your-feature-name`).
3.  Make your changes.
4.  Ensure the code is formatted and passes checks:
    ```bash
    cargo fmt
    cargo clippy
    ```
5.  Commit your changes and push to your branch.
6.  Open a pull request.

## 📄 License

This project is licensed under either of

-   Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
-   MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.