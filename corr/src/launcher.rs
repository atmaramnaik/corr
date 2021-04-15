
use std::fs::{File, read_to_string, create_dir_all};
use flate2::Compression;
use flate2::write::GzEncoder;
use serde::{Deserialize};
use crate::client::CliDriver;
use std::path::Path;
pub fn build(target:String)-> Result<String, std::io::Error>{
    pack(target)
}
#[derive(Deserialize)]
pub struct Config {
    package:Package
}
#[derive(Deserialize)]
struct Package {
    name: String,
}
fn pack(target:String) -> Result<String, std::io::Error> {
    let toml = format!("{}/jpack.toml",target);
    let mut config:Config = Config {
        package:Package{
            name:"temp".to_string()
        }
    };
    if Path::new(toml.as_str()).exists() {
        config = toml::from_str(read_to_string(toml).unwrap().as_str()).unwrap();
    }
    create_dir_all(format!("{}/build",target));
    let result = format!("{}/build/{}.jpack",target,config.package.name.clone());
    let tar_gz = File::create(result.clone())?;
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut tar = tar::Builder::new(enc);
    tar.append_dir_all("./src", "./src")?;
    Ok(result)
}
pub async fn run(target:String,journey:String){
    CliDriver::run(target,journey).await;
}