fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../proto/psl.proto")?;
    tonic_build::compile_protos("../proto/gdp.proto")?;
    Ok(())
}
