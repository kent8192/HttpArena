//! Environment-aware project settings.

use reinhardt::conf::settings::builder::SettingsBuilder;
use reinhardt::conf::settings::profile::Profile;
use reinhardt::conf::settings::sources::{DefaultSource, HighPriorityEnvSource, TomlFileSource};
use reinhardt::settings;
use std::env;

#[settings(core: CoreSettings | contacts: ContactSettings)]
pub struct ProjectSettings;

pub fn get_settings() -> ProjectSettings {
    let profile_name = env::var("REINHARDT_ENV").unwrap_or_else(|_| "local".to_string());
    let settings_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("settings");

    SettingsBuilder::new()
        .profile(Profile::parse(&profile_name))
        .add_source(DefaultSource::new())
        .add_source(TomlFileSource::new(settings_dir.join("base.toml")))
        .add_source(TomlFileSource::new(
            settings_dir.join(format!("{profile_name}.toml")),
        ))
        .add_source(HighPriorityEnvSource::new().with_prefix("REINHARDT_"))
        .build_composed::<ProjectSettings>()
        .expect("failed to load Reinhardt project settings")
}
