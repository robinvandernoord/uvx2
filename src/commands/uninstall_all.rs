use crate::cli::{Process, UninstallAllOptions};
use crate::commands::list::list_packages;
use crate::commands::uninstall::uninstall_package;
use crate::metadata::LoadMetadataConfig;
use owo_colors::OwoColorize;

pub async fn uninstall_all(
    force: bool,
    venv_names: &[String],
) -> Result<(), String> {
    let mut all_ok = true;

    for meta in list_packages(&LoadMetadataConfig::none(), Some(venv_names)).await? {
        match uninstall_package(&meta.name, force).await {
            Ok(msg) => {
                println!("{msg}");
            },
            Err(msg) => {
                eprintln!("{}", msg.red());
                all_ok = false;
            },
        }
    }

    if all_ok {
        Ok(())
    } else {
        Err(String::from(
            "⚠️ Not all packages were properly uninstalled!",
        ))
    }
}

impl Process for UninstallAllOptions {
    async fn process(self) -> Result<i32, String> {
        match uninstall_all(self.force, &self.venv_names).await {
            Ok(()) => Ok(0),
            Err(msg) => Err(msg),
        }
    }
}
