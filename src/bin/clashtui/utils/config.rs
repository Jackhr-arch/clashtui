use anyhow::Result;
use std::fs::File;
use std::path::{Path, PathBuf};

use clashtui::{
    backend::config::{Basic, Service},
    profile::map::ProfileDataBase,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ConfigFile {
    pub basic: Basic,
    pub service: Service,
    pub timeout: Option<u64>,
}
impl ConfigFile {
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let fp = File::create(path)?;
        serde_yaml::to_writer(fp, &self)?;
        Ok(())
    }
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let fp = File::open(path)?;
        Ok(serde_yaml::from_reader(fp)?)
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct DataFile {
    pub profiles: ProfileDataBase,
    pub current_profile: String,
}
impl DataFile {
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let fp = File::create(path)?;
        serde_yaml::to_writer(fp, &self)?;
        Ok(())
    }
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let fp = File::open(path)?;
        Ok(serde_yaml::from_reader(fp)?)
    }
}

pub fn init_config<P: AsRef<Path>>(config_dir: P) -> Result<()> {
    use crate::consts::{BASIC_FILE, CONFIG_FILE, DATA_FILE};
    use std::fs;

    let template_dir = config_dir.as_ref().join("templates");
    let profile_dir = config_dir.as_ref().join("profiles");
    let basic_path = config_dir.as_ref().join(BASIC_FILE);
    let config_path = config_dir.as_ref().join(CONFIG_FILE);
    let data_path = config_dir.as_ref().join(DATA_FILE);

    fs::create_dir_all(&config_dir)?;

    BasicInfo::default().to_file(basic_path)?;
    ConfigFile::default().to_file(config_path)?;
    DataFile::default().to_file(data_path)?;

    fs::create_dir(template_dir)?;
    fs::create_dir(profile_dir)?;

    // fs::write(config_dir.join(BASIC_FILE), DEFAULT_BASIC_CLASH_CFG_CONTENT)?;
    Ok(())
}

pub fn load_config<P: AsRef<Path>>(config_dir: P) -> Result<BuildConfig> {
    use crate::consts::{BASIC_FILE, CONFIG_FILE, DATA_FILE};

    let template_dir = config_dir.as_ref().join("templates");
    let profile_dir = config_dir.as_ref().join("profiles");
    let basic_path = config_dir.as_ref().join(BASIC_FILE);
    let config_path = config_dir.as_ref().join(CONFIG_FILE);
    let data_path = config_dir.as_ref().join(DATA_FILE);

    let cfg = ConfigFile::from_file(config_path)?;
    let data = DataFile::from_file(data_path)?;
    let raw = BasicInfo::get_raw(basic_path)?;
    let base = BasicInfo::from_raw(raw.clone())?;

    Ok(BuildConfig {
        cfg,
        basic: base,
        base_raw: raw,
        data,
        profile_dir,
        template_dir,
    })
}

pub struct BuildConfig {
    pub cfg: ConfigFile,
    pub basic: BasicInfo,
    pub base_raw: serde_yaml::Mapping,
    pub data: DataFile,
    pub profile_dir: PathBuf,
    pub template_dir: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
/// Get necessary info
pub struct BasicInfo {
    #[serde(rename = "external-controller")]
    pub external_controller: String,
    #[serde(rename = "mixed-port")]
    pub mixed_port: Option<u32>,
    pub port: Option<u32>,
    #[serde(rename = "socks-port")]
    pub socks_port: Option<u32>,
    pub secret: Option<String>,
    #[serde(rename = "global-ua")]
    pub global_ua: Option<String>,
}
impl Default for BasicInfo {
    fn default() -> Self {
        Self {
            external_controller: "127.0.0.1:9090".to_string(),
            mixed_port: Some(7890),
            port: None,
            socks_port: None,
            secret: None,
            global_ua: None,
        }
    }
}
impl BasicInfo {
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let fp = File::create(path)?;
        Ok(serde_yaml::to_writer(fp, &self)?)
    }
    // pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
    //     let fp = File::open(path)?;
    //     Ok(serde_yaml::from_reader(fp)?)
    // }
    pub fn get_raw<P: AsRef<Path>>(path: P) -> Result<serde_yaml::Mapping> {
        let fp = File::open(path)?;
        Ok(serde_yaml::from_reader(fp)?)
    }
    pub fn from_raw(raw: serde_yaml::Mapping) -> Result<Self> {
        Ok(serde_yaml::from_value(serde_yaml::Value::Mapping(raw))?)
    }
}
impl BasicInfo {
    pub fn build(self) -> Result<(String, String, Option<String>, Option<String>)> {
        use crate::consts::{BASIC_FILE, LOCALHOST};
        let BasicInfo {
            mut external_controller,
            mixed_port,
            port,
            socks_port,
            secret,
            global_ua,
        } = self;

        if external_controller.starts_with("0.0.0.0") {
            external_controller = format!(
                "127.0.0.1{}",
                external_controller.strip_prefix("0.0.0.0").unwrap()
            );
        }
        external_controller = format!("http://{external_controller}");
        let proxy_addr = match mixed_port
            .or(port)
            .map(|p| format!("http://{LOCALHOST}:{p}"))
        {
            Some(s) => s,
            None => socks_port
                .map(|p| format!("socks5://{LOCALHOST}:{p}"))
                .ok_or(anyhow::anyhow!(
                    "failed to load proxy_addr from {BASIC_FILE}"
                ))?,
        };
        Ok((external_controller, proxy_addr, secret, global_ua))
    }
}
