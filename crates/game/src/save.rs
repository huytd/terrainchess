//! Profile state persistence to file (native) or localStorage (web).

use tc_run::Profile;

#[cfg(not(target_arch = "wasm32"))]
const SAVE_PATH: &str = "terrainchess_profile.ron";

#[cfg(target_arch = "wasm32")]
const STORAGE_KEY: &str = "terrainchess.profile";

/// Load the saved profile, if present and valid.
pub fn load() -> Option<Profile> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        match std::fs::read_to_string(SAVE_PATH) {
            Ok(content) => match Profile::from_ron(&content) {
                Ok(profile) => Some(profile),
                Err(e) => {
                    bevy::log::warn!("failed to deserialize profile from {SAVE_PATH}: {e}");
                    None
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => {
                bevy::log::warn!("failed to read {SAVE_PATH}: {e}");
                None
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        let storage = get_storage()?;
        match storage.get_item(STORAGE_KEY) {
            Ok(Some(ron_str)) => match Profile::from_ron(&ron_str) {
                Ok(profile) => Some(profile),
                Err(e) => {
                    bevy::log::warn!("failed to deserialize profile from localStorage: {e}");
                    None
                }
            },
            Ok(None) => None,
            Err(e) => {
                bevy::log::warn!("failed to read from localStorage: {e:?}");
                None
            }
        }
    }
}

/// Store the current profile.
pub fn store(profile: &Profile) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        match profile.to_ron() {
            Ok(ron_str) => {
                if let Err(e) = std::fs::write(SAVE_PATH, ron_str) {
                    bevy::log::warn!("failed to write {SAVE_PATH}: {e}");
                }
            }
            Err(e) => {
                bevy::log::warn!("failed to serialize profile to ron: {e}");
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        let Some(storage) = get_storage() else { return };
        match profile.to_ron() {
            Ok(ron_str) => {
                if let Err(e) = storage.set_item(STORAGE_KEY, &ron_str) {
                    bevy::log::warn!("failed to write to localStorage: {e:?}");
                }
            }
            Err(e) => {
                bevy::log::warn!("failed to serialize profile to ron: {e}");
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn get_storage() -> Option<web_sys::Storage> {
    let window = match web_sys::window() {
        Some(w) => w,
        None => {
            bevy::log::warn!("no window object available");
            return None;
        }
    };
    match window.local_storage() {
        Ok(Some(storage)) => Some(storage),
        Ok(None) => {
            bevy::log::warn!("localStorage is not available");
            None
        }
        Err(e) => {
            bevy::log::warn!("failed to access localStorage: {e:?}");
            None
        }
    }
}
