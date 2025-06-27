#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

# Define paths and URLs
MODEL_DIR="model"
ONNX_URL="https://huggingface.co/onnx-community/nsfw_image_detection-ONNX/resolve/main/onnx/model.onnx?download=true"
ONNX_PATH="${MODEL_DIR}/model.onnx"
CONFIG_SOURCE_PATH="preprocessor_config.json"
CONFIG_DEST_PATH="${MODEL_DIR}/preprocessor_config.json"

# 1. Create the model directory
echo "Creating model directory at ${MODEL_DIR}..."
mkdir -p "$MODEL_DIR"

# 2. Download the ONNX model if it doesn't exist
if [ -f "$ONNX_PATH" ]; then
    echo "ONNX model already exists. Skipping download."
else
    echo "Downloading ONNX model to ${ONNX_PATH}..."
    if command -v curl &> /dev/null; then
        curl -L "$ONNX_URL" -o "$ONNX_PATH"
    elif command -v wget &> /dev/null; then
        wget "$ONNX_URL" -O "$ONNX_PATH"
    else
        echo "Error: Neither curl nor wget is available. Please download the model manually."
        exit 1
    fi
    echo "Model downloaded successfully."
fi


# 3. Copy the preprocessor config if it doesn't exist
if [ -f "$CONFIG_DEST_PATH" ]; then
    echo "Preprocessor config already exists. Skipping copy."
else
    if [ -f "$CONFIG_SOURCE_PATH" ]; then
        echo "Copying preprocessor config to ${CONFIG_DEST_PATH}..."
        cp "$CONFIG_SOURCE_PATH" "$CONFIG_DEST_PATH"
    else
        echo "Error: Source config file '${CONFIG_SOURCE_PATH}' not found."
        exit 1
    fi
fi


# 4. Final verification
echo ""
echo "✅ Model setup complete."
echo "Your directory structure is:"
ls -l "$MODEL_DIR"