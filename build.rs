fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("cargo:rerun-if-changed=proto");

  let protos: Vec<_> = std::fs::read_dir("proto")?
    .filter_map(|entry| entry.ok())
    .map(|entry| entry.path())
    .filter(|path| path.extension().is_some_and(|ext| ext == "proto"))
    .collect();

  tonic_prost_build::configure()
    .compile_protos(&protos, &[std::path::PathBuf::from("proto")])?;

  Ok(())
}
