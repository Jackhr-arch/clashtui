mod blob_file;
mod config;
mod config_struct;
mod conn;
mod control;
pub mod local_config;

use super::*;
#[allow(unused)]
pub use config_struct::{ClashConfig, LogLevel, Mode, TunConfig, TunStack};
#[cfg(feature = "connection-tab")]
pub use conn::{Conn, ConnInfo, ConnMetaData};

#[derive(Debug)]
pub struct ClashUtil {
    api: String,
    secret: Option<String>,
    ua: Option<String>,
    // timeout: u64,
    pub proxy_addr: String,
}

impl ClashUtil {
    pub fn new(
        controller_api: String,
        secret: Option<String>,
        proxy_addr: String,
        ua: Option<String>,
        timeout: Option<u64>,
    ) -> Self {
        TIMEOUT.set(timeout.unwrap_or(DEFAULT_TIMEOUT)).unwrap();
        Self {
            api: controller_api,
            secret,
            ua,
            proxy_addr,
        }
    }
    fn request(
        &self,
        method: minreq::Method,
        sub_url: &str,
        payload: Option<String>,
    ) -> Result<minreq::Response, minreq::Error> {
        let mut req = minreq::Request::new(method, self.api.to_owned() + sub_url);
        if let Some(kv) = payload {
            req = req.with_body(kv);
        }
        if let Some(s) = self.secret.as_ref() {
            req = req.with_header(headers::AUTHORIZATION, format!("Bearer {s}"));
        }
        req.with_timeout(Self::timeout()).send()
    }
    fn timeout() -> u64 {
        *TIMEOUT.get().unwrap()
    }

    #[cfg(test)]
    /// used for test
    fn build_test() -> Self {
        Self::new(
            "http://127.0.0.1:9090".to_string(),
            Some("test".to_owned()),
            "http://127.0.0.1:7890".to_string(),
            None,
            None,
        )
    }
}
