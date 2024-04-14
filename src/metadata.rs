use crate::helpers::ResultToString;
use crate::symlinks::check_symlink;
use crate::uv::{uv_get_installed_version, uv_venv, Helpers};
use itertools::Itertools;
use owo_colors::OwoColorize;
use pep508_rs::Requirement;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use uv_interpreter::PythonEnvironment;

const BIN_DIR: &'static str = ".local/bin";
const WORK_DIR: &'static str = ".local/uvx";
const INDENT: &'static str = "    ";

pub fn get_home_dir() -> PathBuf {
    return home::home_dir().expect("Failed to get home directory");
}

pub fn get_bin_dir() -> PathBuf {
    let home_dir = get_home_dir();
    return home_dir.join(BIN_DIR);
}

pub fn get_work_dir() -> PathBuf {
    let home_dir = get_home_dir();
    return home_dir.join(WORK_DIR);
}

pub fn get_venv_dir() -> PathBuf {
    let work_dir = get_work_dir();
    return work_dir.join("venvs");
}

#[derive(Debug, PartialEq, Deserialize, Serialize, dbg_pls::DebugPls)]
pub struct Metadata {
    // order is important!!
    pub name: String,
    #[serde(default)]
    pub scripts: HashMap<String, bool>,
    pub install_spec: String,
    #[serde(default)]
    pub extras: HashSet<String>,
    #[serde(default)]
    pub requested_version: String,
    pub installed_version: String,
    pub python: String,
    pub python_raw: String,

    #[serde(default)]
    pub injected: HashSet<String>,
}

impl Metadata {
    pub fn new(name: &str) -> Metadata {
        return Metadata {
            name: name.to_string(),
            scripts: HashMap::new(),
            install_spec: name.to_string(),
            extras: HashSet::new(),
            requested_version: String::new(),
            installed_version: String::new(),
            python: String::new(),
            python_raw: String::new(),
            injected: HashSet::new(),
        };
    }

    pub fn find(req: &Requirement) -> Metadata {
        let mut empty = Metadata::new(&req.name.to_string());

        empty.installed_version = uv_get_installed_version(&req.name, None).unwrap_or_default();

        empty.fill(None);
        return empty;
    }

    /// try to guess/deduce some values
    pub fn fill(&mut self, maybe_venv: Option<&PythonEnvironment>) -> Option<()> {
        let _v: PythonEnvironment;

        let venv = match maybe_venv {
            Some(v) => v,
            None => match uv_venv(None) {
                Some(v) => {
                    _v = v;
                    &_v
                }
                None => return None,
            },
        };

        let python_info = venv.interpreter().markers();

        if self.install_spec == "" {
            self.install_spec = String::from(&self.name);
        }

        if self.python == "" {
            self.python = format!(
                "{} {}",
                python_info.platform_python_implementation, python_info.python_full_version
            )
        }

        if self.python_raw == "" {
            self.python_raw = venv.stdlib_as_string();
        }

        return Some(());
    }

    pub async fn for_dir(dirname: &PathBuf, recheck_scripts: bool) -> Option<Metadata> {
        let meta_path = dirname.join(".metadata");

        return Metadata::for_file(&meta_path, recheck_scripts).await;
    }

    pub async fn for_file(filename: &PathBuf, recheck_scripts: bool) -> Option<Metadata> {
        let result = load_metadata(filename, recheck_scripts).await;
        return result.ok();
    }

    pub async fn save(&self, dirname: &PathBuf) -> Result<(), String> {
        let meta_path = dirname.join(".metadata");
        return store_metadata(&meta_path, &self).await;
    }

    pub async fn check_scripts(&mut self, venv_path: &Path) {
        for (key, value) in self.scripts.iter_mut() {
            *value = check_symlink(key, venv_path).await;
        }
    }

    pub fn format_short(&self) -> String {
        return format!("- {} {}", self.name, self.installed_version.cyan());
    }

    pub fn format_human(&self) -> String {
        let mut result = format!("- {}", self.name); // todo: colorized extra's (+ install spec?)

        if self.extras.len() > 0 {
            result.push_str(&format!(
                "[{}]",
                self.extras
                    .iter()
                    .map(|k| format!("'{}'", k.green()))
                    .join(",")
            ));
        }

        if self.requested_version.len() > 0 {
            result.push_str(&format!(" {}", self.requested_version.cyan()));
        }

        result.push_str("\n");

        result.push_str(&format!(
            "{}Installed Version: {} on {}.\n",
            INDENT,
            self.installed_version.cyan(),
            self.python.bright_blue()
        ));

        let formatted_injects = self
            .injected
            .iter()
            .map(|k| format!("'{}'", k.green()))
            .join(", ");
        result.push_str(&format!(
            "{}Injected Packages: {}\n",
            INDENT, formatted_injects
        ));

        let formatted_scripts = self
            .scripts
            .iter()
            .map(|(key, value)| {
                if *value {
                    key.green().to_string()
                } else {
                    key.red().to_string()
                }
            })
            .join(" | ");
        result.push_str(&format!("{}Scripts: {}", INDENT, formatted_scripts));

        return result;
    }
}

pub async fn load_metadata(filename: &Path, recheck_scripts: bool) -> Result<Metadata, String> {
    // Open the msgpack file
    let mut file = File::open(filename).await.map_err_to_string()?;

    let mut buf = Vec::new();
    file.read_to_end(&mut buf).await.map_err_to_string()?;

    // Read the contents of the file into a Metadata struct
    let mut metadata: Metadata = rmp_serde::decode::from_slice(&buf[..]).map_err_to_string()?;

    if recheck_scripts {
        let _ = &metadata.check_scripts(&filename.parent().unwrap()).await;
    }

    Ok(metadata)
}

pub async fn store_metadata(filename: &Path, metadata: &Metadata) -> Result<(), String> {
    // Open the msgpack file
    let mut file = File::create(filename).await.map_err_to_string()?;

    // Read the contents of the file into a Metadata struct
    let bytes = rmp_serde::encode::to_vec(metadata).map_err_to_string()?;

    file.write_all(&bytes).await.map_err_to_string()?;

    Ok(())
}