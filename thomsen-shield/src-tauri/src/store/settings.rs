use crate::error::AppResult;
use crate::store::models::Settings;

use super::{load_or_default, paths, save_atomic};

pub fn load() -> AppResult<Settings> {
    load_or_default(&paths::settings_path()?)
}

pub fn save(settings: &Settings) -> AppResult<()> {
    save_atomic(&paths::settings_path()?, settings)
}
