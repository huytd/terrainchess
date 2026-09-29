//! Run state persistence to file (native) or localStorage (web).

use tc_run::RunState;

#[cfg(not(target_arch = "wasm32"))]
const SAVE_PATH: &str = "terrainchess_run.ron";

#[cfg(target_arch = "wasm32")]
const STORAGE_KEY: &str = "terrainchess.run";

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

#[cfg(not(target_arch = "wasm32"))]
const CAMPAIGN_SAVE_PATH: &str = "terrainchess_campaign.ron";

#[cfg(target_arch = "wasm32")]
const CAMPAIGN_STORAGE_KEY: &str = "terrainchess.campaign";

pub fn load_campaign() -> Option<tc_world::World> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        match std::fs::read_to_string(CAMPAIGN_SAVE_PATH) {
            Ok(content) => match tc_world::World::from_ron(&content) {
                Ok(state) => Some(state),
                Err(e) => {
                    bevy::log::warn!("failed to deserialize campaign from {CAMPAIGN_SAVE_PATH}: {e}");
                    None
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => {
                bevy::log::warn!("failed to read {CAMPAIGN_SAVE_PATH}: {e}");
                None
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        let storage = get_storage()?;
        match storage.get_item(CAMPAIGN_STORAGE_KEY) {
            Ok(Some(ron_str)) => match tc_world::World::from_ron(&ron_str) {
                Ok(state) => Some(state),
                Err(e) => {
                    bevy::log::warn!("failed to deserialize campaign from localStorage: {e}");
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

pub fn has_campaign_save() -> bool {
    load_campaign().is_some()
}

pub fn store_campaign(state: &tc_world::World) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        match state.to_ron() {
            Ok(ron_str) => {
                if let Err(e) = std::fs::write(CAMPAIGN_SAVE_PATH, ron_str) {
                    bevy::log::warn!("failed to write {CAMPAIGN_SAVE_PATH}: {e}");
                }
            }
            Err(e) => {
                bevy::log::warn!("failed to serialize campaign to ron: {e}");
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        let Some(storage) = get_storage() else { return };
        match state.to_ron() {
            Ok(ron_str) => {
                if let Err(e) = storage.set_item(CAMPAIGN_STORAGE_KEY, &ron_str) {
                    bevy::log::warn!("failed to write to localStorage: {e:?}");
                }
            }
            Err(e) => {
                bevy::log::warn!("failed to serialize campaign to ron: {e}");
            }
        }
    }
}

#[allow(dead_code)]
pub fn clear_campaign() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Err(e) = std::fs::remove_file(CAMPAIGN_SAVE_PATH) {
            if e.kind() != std::io::ErrorKind::NotFound {
                bevy::log::warn!("failed to remove {CAMPAIGN_SAVE_PATH}: {e}");
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        if let Some(storage) = get_storage()
            && let Err(e) = storage.remove_item(CAMPAIGN_STORAGE_KEY)
        {
            bevy::log::warn!("failed to remove from localStorage: {e:?}");
        }
    }
}
