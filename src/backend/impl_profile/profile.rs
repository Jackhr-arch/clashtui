use super::extract_domain;
use std::{fs::File, path::PathBuf};

#[derive(Clone)]
pub struct Profile {
    pub name: String,
    pub dtype: ProfileType,
}
impl Default for Profile {
    fn default() -> Self {
        Self {
            name: "Unknown".into(),
            dtype: ProfileType::File,
        }
    }
}

#[derive(Clone)]
pub struct LocalProfile {
    pub name: String,
    pub dtype: ProfileType,
    pub path: PathBuf,
    pub content: Option<serde_yml::Mapping>,
}
impl Default for LocalProfile {
    fn default() -> Self {
        Self {
            name: "base".into(),
            dtype: ProfileType::File,
            path: Default::default(),
            content: Default::default(),
        }
    }
}

impl LocalProfile {
    /// Returns the atime of this [`LocalProfile`].
    ///
    /// Errors are ignored and return will be replaced with [None]
    pub fn atime(&self) -> Option<core::time::Duration> {
        pub fn get_modify_time<P>(file_path: P) -> std::io::Result<std::time::SystemTime>
        where
            P: AsRef<std::path::Path>,
        {
            let file = std::fs::metadata(file_path)?;
            if file.is_file() {
                file.modified()
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Not a file",
                ))
            }
        }
        let now = std::time::SystemTime::now();
        get_modify_time(&self.path)
            .ok()
            .and_then(|file| now.duration_since(file).ok())
    }
    /// merge `basic_clash_config` to `self::content`,
    /// all items in `basic_clash_config` will be overwritten to `self::content`
    /// except for the value is a sequence, it will be appended to the original value
    ///
    /// Note: need to call [`LocalProfile::sync_from_disk`] before call this
    pub fn merge(&mut self, basic_clash_config: &serde_yml::Mapping) -> anyhow::Result<()> {
        if self.content.is_none() || basic_clash_config.is_empty() {
            anyhow::bail!("failed to merge: one of the input content is none");
        }
        let map = self.content.as_mut().unwrap();
        for (key, value) in basic_clash_config.iter() {
            if let Some(serde_yml::Value::Sequence(mut old_value)) = map.swap_remove(key) {
                if let Some(v) = value.as_sequence() {
                    if key == "rules" {
                        // get MATCH
                        let end = old_value.remove(old_value.len() - 1);
                        old_value.extend(v.iter().cloned());
                        old_value.push(end);
                    } else {
                        old_value.extend(v.iter().cloned())
                    }
                }
                map.insert(key.clone(), serde_yml::Value::Sequence(old_value));
            } else {
                map.insert(key.clone(), value.clone());
            }
        }
        Ok(())
    }
    /// sync the content to disk by [`LocalProfile::path`]
    pub fn sync_to_disk(self) -> anyhow::Result<()> {
        let LocalProfile { path, content, .. } = self;
        let fp = File::create(path)
            .map_err(|e| anyhow::anyhow!("Failed to write clash config file: {e}"))?;
        Ok(serde_yml::to_writer(fp, &content)?)
    }
    pub fn from_pf(pf: Profile, path: std::path::PathBuf) -> Self {
        let Profile { name, dtype } = pf;
        Self {
            name,
            dtype,
            path,
            content: None,
        }
    }
    /// sync the content from disk by [`LocalProfile::path`]
    pub fn sync_from_disk(&mut self) -> anyhow::Result<()> {
        if self.path.is_file() {
            let fp = File::open(&self.path)?;
            self.content = serde_yml::from_reader(fp).ok();
        }
        Ok(())
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub enum ProfileType {
    /// local import
    File,
    /// download url
    Url(String),
    /// generated by template
    Generated(String),
    Github {
        url: String,
        token: String,
    },
    GitLab {
        url: String,
        token: String,
    },
}
impl ProfileType {
    /// if [ProfileType::is_upgradable], return [Some]
    pub fn get_domain(&self) -> Option<String> {
        match self {
            ProfileType::File => None,
            ProfileType::Url(url) => extract_domain(url).map(|s| s.to_owned()),
            ProfileType::Generated(name) => Some(format!("From template {name}")),
            ProfileType::Github { url, token: _ } => extract_domain(url).map(|s| s.to_owned()),
            ProfileType::GitLab { url, token: _ } => extract_domain(url).map(|s| s.to_owned()),
        }
    }
}
