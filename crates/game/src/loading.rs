//! Loading state and ready gating for assets and UI.

use bevy::asset::LoadState;
use bevy::prelude::*;

use crate::atlas::Atlas;

/// Application lifecycle states.
#[derive(States, Default, Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum AppState {
    #[default]
    Loading,
    Ready,
}

pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_systems(OnEnter(AppState::Loading), notify_loading_start)
            .add_systems(Update, check_assets.run_if(in_state(AppState::Loading)))
            .add_systems(Update, notify_ready_frame.run_if(in_state(AppState::Ready)));
    }
}

fn notify_loading_start() {
    call_js_status("Loading art...");
}

fn check_assets(
    asset_server: Res<AssetServer>,
    atlas: Res<Atlas>,
    mut next_state: ResMut<NextState<AppState>>,
    mut failed: Local<bool>,
) {
    if *failed {
        return;
    }
    match asset_server.load_state(&atlas.image) {
        LoadState::Failed(err) => {
            *failed = true;
            bevy::log::error!("Failed to load atlas: {err:?}");
            call_js_status("Failed to load art");
        }
        _ => {
            if asset_server.is_loaded_with_dependencies(&atlas.image) {
                next_state.set(AppState::Ready);
            }
        }
    }
}

/// On entering Ready, wait one frame so the first frame with UI is rendered,
/// then notify window.terrainchessReady().
fn notify_ready_frame(mut frame_count: Local<u32>, mut called: Local<bool>) {
    if *called {
        return;
    }
    *frame_count += 1;
    if *frame_count >= 2 {
        *called = true;
        call_js_ready();
    }
}

#[cfg(target_arch = "wasm32")]
pub fn call_js_ready() {
    use wasm_bindgen::JsCast;
    if let Some(window) = web_sys::window()
        && let Ok(func_val) =
            js_sys::Reflect::get(&window, &wasm_bindgen::JsValue::from_str("terrainchessReady"))
        && let Some(func) = func_val.dyn_ref::<js_sys::Function>()
    {
        let _ = func.call0(&window);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn call_js_ready() {}

#[cfg(target_arch = "wasm32")]
pub fn call_js_status(text: &str) {
    use wasm_bindgen::JsCast;
    if let Some(window) = web_sys::window()
        && let Ok(func_val) =
            js_sys::Reflect::get(&window, &wasm_bindgen::JsValue::from_str("terrainchessStatus"))
        && let Some(func) = func_val.dyn_ref::<js_sys::Function>()
    {
        let arg = wasm_bindgen::JsValue::from_str(text);
        let _ = func.call1(&window, &arg);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn call_js_status(_text: &str) {}
