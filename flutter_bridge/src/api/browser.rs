use ui_ffi_common::pm_core::{BrowserType, TabId};

pub async fn close_tab(tab_id: &TabId) {}

pub async fn activate_tab(tab_id: &TabId) {}

pub async fn create_tab(url: &str, browser: BrowserType) -> TabId {
    TabId::new()
}

pub async fn get_connected_browsers() -> Vec<BrowserType> {
    Vec::new()
}
