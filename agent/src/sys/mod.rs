#[cfg(target_os = "macos")]
pub mod macos_audio;

#[cfg(target_os = "macos")]
pub mod macos_session;

#[cfg(target_os = "macos")]
pub mod macos_power;

#[cfg(target_os = "windows")]
pub mod windows_service;
