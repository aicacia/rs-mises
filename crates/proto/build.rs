use std::{env, error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
  #[allow(unused_mut)]
  let mut config = tonic_prost_build::configure();

  #[cfg(feature = "file-descriptor-set")]
  {
    config = config.file_descriptor_set_path(PathBuf::from(env::var("OUT_DIR")?).join("mises.bin"));
  }

  config.compile_protos(&["proto/mises.proto"], &["proto"])?;

  Ok(())
}
