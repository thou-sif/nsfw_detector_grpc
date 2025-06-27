fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()); // Get OUT_DIR
    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("nsfw_detector_descriptor.bin")) // Configure path for descriptor
        .compile_protos(&["proto/nsfw_detector.proto"], &["proto"])?;
    Ok(())
}
