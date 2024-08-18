pub(crate) const DEFAULT_BASIC_CLASH_CFG_CONTENT: &str = r#"mixed-port: 7890
mode: rule
log-level: info
external-controller: 127.0.0.1:9090"#;
pub(crate) const BASIC_FILE: &str = "basic_clash_config.yaml";
pub(crate) const HOST: &str = "127.0.0.1";
pub mod const_err {
    pub const ERR_PATH_UTF_8: &str = "path is not utf-8 form";
}
pub(crate) const CONFIG_FILE: &str = "config.yaml";
pub(crate) const DATA_FILE: &str = "clashtui.conf";
pub(crate) const TMP_PATH: &str = "/tmp/clashtui_mihomo_config_file.tmp";
pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"));
