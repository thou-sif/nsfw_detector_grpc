fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .compile_protos(
            &["proto/nsfw_detector.proto"], // Path to your .proto file
            &["proto"],                     // Directory to search for imports (if any)
        )?;
    Ok(())
}
