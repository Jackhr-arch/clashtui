use std::{
    fs::File,
    path::{Path, PathBuf},
};

pub mod map;

pub use map::ProfileType;

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

const FILTER: [&str; 6] = [
    "proxy-groups",
    "proxy-providers",
    "proxies",
    "sub-rules",
    "rules",
    "rule-providers",
];
#[derive(Clone)]
pub struct LocalProfile {
    pub name: String,
    pub dtype: ProfileType,
    pub path: PathBuf,
    pub content: Option<serde_yaml::Mapping>,
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
    pub fn new<P: AsRef<Path>>(path: P, profile: Profile) -> anyhow::Result<Self> {
        let Profile { name, dtype } = profile;
        let fp = File::open(path.as_ref())?;
        let content = serde_yaml::from_reader(fp)?;
        Ok(Self {
            name,
            dtype,
            path: path.as_ref().into(),
            content,
        })
    }

    /// merge `base` into [`LocalProfile::content`],
    /// using [`FILTER`]
    ///
    /// Note: need to call [`LocalProfile::sync_from_disk`] before call this
    pub fn merge(&mut self, base: &LocalProfile) -> anyhow::Result<()> {
        if self.content.is_none() || base.content.is_none() {
            // this should be handled at develop time
            panic!("one of the input content is none");
        }

        FILTER
            .into_iter()
            .filter(|s| base.content.as_ref().unwrap().contains_key(s))
            .map(|key| (key, base.content.as_ref().unwrap().get(key).unwrap()))
            .for_each(|(k, v)| {
                self.content.as_mut().unwrap().insert(k.into(), v.clone());
            });
        Ok(())
    }
    /// sync the content to disk by [`LocalProfile::path`]
    pub fn sync_to_disk(self) -> anyhow::Result<()> {
        let LocalProfile { path, content, .. } = self;
        let fp = File::create(path)?;
        Ok(serde_yaml::to_writer(fp, &content)?)
    }
    /// sync the content from disk by [`LocalProfile::path`]
    pub fn sync_from_disk(&mut self) -> anyhow::Result<()> {
        let fp = File::open(&self.path)?;
        self.content = serde_yaml::from_reader(fp)?;
        Ok(())
    }
}