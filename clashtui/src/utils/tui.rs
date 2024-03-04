use core::cell::RefCell;
use core::str::FromStr as _;
use std::{
    fs::File,
    io::{Error, Read},
    path::{Path, PathBuf},
};

use super::{
    config::{CfgError, ClashTuiConfig, ErrKind},
    state::_State,
};
use api::{ClashConfig, ClashUtil, Resp};

const BASIC_FILE: &str = "basic_clash_config.yaml";

pub struct ClashTuiUtil {
    pub clashtui_dir: PathBuf,
    pub(super) profile_dir: PathBuf,

    clash_api: ClashUtil,
    pub tui_cfg: ClashTuiConfig,
    clash_remote_config: RefCell<Option<ClashConfig>>,
}

// Misc
impl ClashTuiUtil {
    pub fn new(
        clashtui_dir: &PathBuf,
        profile_dir: &Path,
        is_inited: bool,
    ) -> (Self, Vec<CfgError>) {
        let ret = load_app_config(clashtui_dir, is_inited);
        let mut err_track = ret.3;
        let clash_api = ClashUtil::new(ret.1, ret.2);
        let cur_remote = match clash_api.config_get() {
            Ok(v) => v,
            Err(_) => String::new(),
        };
        let remote = match ClashConfig::from_str(cur_remote.as_str()) {
            Ok(v) => Some(v),
            Err(_) => {
                err_track.push(CfgError::new(
                    ErrKind::LoadClashConfig,
                    "Fail to load config from clash core. Is it Running?".to_string(),
                ));
                log::warn!("Fail to connect to clash. Is it Running?");
                None
            }
        };
        (
            Self {
                clashtui_dir: clashtui_dir.clone(),
                profile_dir: profile_dir.to_path_buf(),
                clash_api,
                tui_cfg: ret.0,
                clash_remote_config: RefCell::new(remote),
            },
            err_track,
        )
    }

    fn _update_state(
        &self,
        new_pf: Option<String>,
        new_mode: Option<String>,
    ) -> (String, Option<api::Mode>, Option<api::TunStack>, String) {
        if let Some(v) = new_mode {
            let load = format!(r#"{{"mode": "{}"}}"#, v);
            let _ = self
                .clash_api
                .config_patch(load)
                .map_err(|e| log::error!("Patch Errr: {}", e));
        }

        let pf = match new_pf {
            Some(v) => {
                self.tui_cfg.update_profile(&v);
                v
            }
            None => self.tui_cfg.current_profile.borrow().clone(),
        };

        let ver = match self.clash_api.version() {
            Ok(v) => v,
            Err(e) => {
                log::warn!("{}", e);
                "Unknown".to_string()
            }
        };
        if let Err(e) = self.fetch_remote() {
            if e.kind() != std::io::ErrorKind::ConnectionRefused {
                log::warn!("{}", e);
            }
        }
        let (mode, tun) = match self.clash_remote_config.borrow().as_ref() {
            Some(v) => (
                Some(v.mode),
                if v.tun.enable {
                    Some(v.tun.stack)
                } else {
                    None
                },
            ),
            None => (None, None),
        };
        (pf, mode, tun, ver)
    }

    #[cfg(target_os = "windows")]
    pub fn update_state(
        &self,
        new_pf: Option<String>,
        new_mode: Option<String>,
        new_sysp: Option<bool>,
    ) -> _State {
        if let Some(b) = new_sysp {
            let _ = if b {
                super::ipc::enable_system_proxy(&self.clash_api.proxy_addr)
            } else {
                super::ipc::disable_system_proxy()
            };
        }
        let (pf, mode, tun, ver) = self._update_state(new_pf, new_mode);
        let sysp = super::ipc::is_system_proxy_enabled().map_or_else(
            |v| {
                log::error!("{}", v);
                None
            },
            Some,
        );
        _State {
            profile: pf,
            mode,
            tun,
            ver,
            sysproxy: sysp,
        }
    }

    #[cfg(target_os = "linux")]
    pub fn update_state(&self, new_pf: Option<String>, new_mode: Option<String>) -> _State {
        let (pf, mode, tun, ver) = self._update_state(new_pf, new_mode);
        _State {
            profile: pf,
            mode,
            tun,
            ver,
        }
    }

    pub fn fetch_recent_logs(&self, num_lines: usize) -> Vec<String> {
        let log = std::fs::read_to_string(self.clashtui_dir.join("clashtui.log"))
            .unwrap_or_else(|_| String::new());
        log.lines()
            .rev()
            .take(num_lines)
            .map(String::from)
            .collect()
    }
}
// Web
impl ClashTuiUtil {
    fn fetch_remote(&self) -> Result<(), Error> {
        let cur_remote = self.clash_api.config_get()?;
        let remote = ClashConfig::from_str(cur_remote.as_str())
            .map_err(|_| Error::new(std::io::ErrorKind::InvalidData, "Failed to prase str"))?;
        log::debug!("{:#?}", remote);
        self.clash_remote_config.borrow_mut().replace(remote);
        log::debug!("{:#?}", self.clash_remote_config.borrow());
        Ok(())
    }

    pub fn restart_clash(&self) -> Result<String, Error> {
        self.clash_api.restart(None)
    }

    pub fn select_profile(&self, profile_name: &String) -> Result<(), Error> {
        if let Err(err) = self.merge_profile(profile_name) {
            log::error!(
                "Failed to Merge Profile `{}`: {}",
                profile_name,
                err.to_string()
            );
            return Err(Error::new(std::io::ErrorKind::Other, err));
        };
        let body = serde_json::json!({
            "path": self.tui_cfg.clash_cfg_path.as_str(),
            "payload": ""
        })
        .to_string();
        if let Err(err) = self.clash_api.config_reload(body) {
            log::error!(
                "Failed to Patch Profile `{}`: {}",
                profile_name,
                err.to_string()
            );
            return Err(Error::new(std::io::ErrorKind::Other, err));
        };
        Ok(())
    }

    fn merge_profile(&self, profile_name: &String) -> std::io::Result<()> {
        let basic_clash_cfg_path = self.clashtui_dir.join(BASIC_FILE);
        let mut dst_parsed_yaml = parse_yaml(&basic_clash_cfg_path)?;
        let profile_yaml_path = self.get_profile_yaml_path(profile_name)?;
        let profile_parsed_yaml = parse_yaml(&profile_yaml_path).map_err(|e| {
            Error::new(
                e.kind(),
                format!(
                    "Maybe need to update first. Failed to parse {}: {e}",
                    profile_yaml_path.to_str().unwrap()
                ),
            )
        })?;

        if let serde_yaml::Value::Mapping(dst_mapping) = &mut dst_parsed_yaml {
            if let serde_yaml::Value::Mapping(mapping) = &profile_parsed_yaml {
                for (key, value) in mapping.iter() {
                    if let serde_yaml::Value::String(k) = key {
                        match k.as_str() {
                            "proxy-groups" | "proxy-providers" | "proxies" | "sub-rules"
                            | "rules" | "rule-providers" => {
                                dst_mapping.insert(key.clone(), value.clone());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        let final_clash_cfg_file = File::create(&self.tui_cfg.clash_cfg_path)?;
        serde_yaml::to_writer(final_clash_cfg_file, &dst_parsed_yaml)
            .map_err(|e| Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        Ok(())
    }

    pub fn dl_remote_profile(&self, url: &str) -> Result<Resp, Error> {
        self.clash_api.mock_clash_core(url)
    }

    pub fn update_geo(&self) -> Result<String, Error> {
        self.clash_api
            .check_geo_update(None, Path::new(&self.tui_cfg.clash_cfg_dir))
    }
}

pub(super) fn parse_yaml(yaml_path: &Path) -> std::io::Result<serde_yaml::Value> {
    let mut file = File::open(yaml_path)?;
    let mut yaml_content = String::new();
    file.read_to_string(&mut yaml_content)?;
    let parsed_yaml_content: serde_yaml::Value =
        serde_yaml::from_str(yaml_content.as_str()).unwrap();
    Ok(parsed_yaml_content)
}

fn load_app_config(
    clashtui_dir: &PathBuf,
    skip_init_conf: bool,
) -> (ClashTuiConfig, String, String, Vec<CfgError>) {
    let mut err_collect = Vec::new();
    let basic_clash_config_path = Path::new(clashtui_dir).join(BASIC_FILE);
    let basic_clash_config_value: serde_yaml::Value =
        match parse_yaml(basic_clash_config_path.as_path()) {
            Ok(r) => r,
            Err(_) => {
                err_collect.push(CfgError::new(
                    ErrKind::LoadProfileConfig,
                    "Fail to load User Defined Config".to_string(),
                ));
                serde_yaml::Value::Null
            }
        };
    let controller_api = basic_clash_config_value
        .get("external-controller")
        .and_then(|v| {
            format!(
                "http://{}",
                v.as_str().expect("external-controller not str?")
            )
            .into()
        })
        .unwrap_or_else(|| panic!("No external-controller in {BASIC_FILE}"));
    log::debug!("controller_api: {}", controller_api);

    let proxy_addr = get_proxy_addr(&basic_clash_config_value);
    log::debug!("proxy_addr: {}", proxy_addr);

    let configs = if skip_init_conf {
        let config_path = clashtui_dir.join("config.yaml");
        match ClashTuiConfig::from_file(config_path.to_str().unwrap()) {
            Ok(v) => {
                if !v.check() {
                    err_collect.push(CfgError::new(
                        ErrKind::LoadAppConfig,
                        "Some Key Configs are missing, or Default".to_string(),
                    ));
                    log::warn!("Empty Config?");
                    log::debug!("{:?}", v)
                };
                v
            }
            Err(e) => {
                err_collect.push(CfgError::new(
                    ErrKind::LoadAppConfig,
                    "Fail to load configs, using Default".to_string(),
                ));
                log::error!("Unable to load config file. {}", e);
                ClashTuiConfig::default()
            }
        }
    } else {
        ClashTuiConfig::default()
    };

    (configs, controller_api, proxy_addr, err_collect)
}

fn get_proxy_addr(yaml_data: &serde_yaml::Value) -> String {
    let host = "127.0.0.1";
    if let Some(port) = yaml_data.get("mixed-port").and_then(|v| v.as_u64()) {
        return format!("http://{}:{}", host, port);
    }
    if let Some(port) = yaml_data.get("port").and_then(|v| v.as_u64()) {
        return format!("http://{}:{}", host, port);
    }
    if let Some(port) = yaml_data.get("socks-port").and_then(|v| v.as_u64()) {
        return format!("socks5://{}:{}", host, port);
    }
    panic!("No prots in {BASIC_FILE}")
}
