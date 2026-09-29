//! Run state persistence to file (native) or localStorage (web).

use tc_run::RunState;

#[cfg(not(target_arch = "wasm32"))]
const SAVE_PATH: &str = "terrainchess_run.ron";

#[cfg(target_arch = "wasm32")]
const STORAGE_KEY: &str = "terrainchess.run";

/// Load the saved run state, if present and valid.
pub fn load() -> Option<RunState> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        match std::fs::read_to_string(SAVE_PATH) {
            Ok(content) => match RunState::from_ron(&content) {
                Ok(state) => Some(state),
                Err(e) => {
                    bevy::log::warn!("failed to deserialize run from {SAVE_PATH}: {e}");
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
            Ok(Some(ron_str)) => match RunState::from_ron(&ron_str) {
                Ok(state) => Some(state),
                Err(e) => {
                    bevy::log::warn!("failed to deserialize run from localStorage: {e}");
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

/// Returns true if a saved run state exists.
pub fn has_save() -> bool {
    load().is_some()
}

/// Store the current run state.
pub fn store(state: &RunState) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        match state.to_ron() {
            Ok(ron_str) => {
                if let Err(e) = std::fs::write(SAVE_PATH, ron_str) {
                    bevy::log::warn!("failed to write {SAVE_PATH}: {e}");
                }
            }
            Err(e) => {
                bevy::log::warn!("failed to serialize run state to ron: {e}");
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        let Some(storage) = get_storage() else { return };
        match state.to_ron() {
            Ok(ron_str) => {
                if let Err(e) = storage.set_item(STORAGE_KEY, &ron_str) {
                    bevy::log::warn!("failed to write to localStorage: {e:?}");
                }
            }
            Err(e) => {
                bevy::log::warn!("failed to serialize run state to ron: {e}");
            }
        }
    }
}

/// Clear any saved run state.
pub fn clear() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Err(e) = std::fs::remove_file(SAVE_PATH) {
            if e.kind() != std::io::ErrorKind::NotFound {
                bevy::log::warn!("failed to remove {SAVE_PATH}: {e}");
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        if let Some(storage) = get_storage()
            && let Err(e) = storage.remove_item(STORAGE_KEY)
        {
            bevy::log::warn!("failed to remove from localStorage: {e:?}");
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
