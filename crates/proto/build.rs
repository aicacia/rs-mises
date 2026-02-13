fn main() -> Result<(), Box<dyn std::error::Error>> {
  #[allow(unused_mut)]
  let mut config = tonic_prost_build::configure();

  #[cfg(feature = "file-descriptor-set")]
  {
    config = config.file_descriptor_set_path(
      std::path::PathBuf::from(std::env::var("OUT_DIR")?).join("mises.bin"),
    );
  }

  config.compile_protos(&["proto/mises.proto"], &["proto"])?;

  Ok(())
}
