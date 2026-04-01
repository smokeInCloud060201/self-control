#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::error::Result;
use clap::Parser;
use rand::{thread_rng, Rng};

use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

mod app_state;
mod capture;
mod cli;
mod config;
mod controller;
mod core_service;
mod error;
mod gui;
mod models;
mod network;
mod sys;

use cli::Args;
use app_state::AppState;

fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    let args = Args::parse();

    let machine_id = args.machine_id.clone().unwrap_or_else(|| {
        local_ip_address::local_ip()
            .map(|ip| ip.to_string())
            .unwrap_or_else(|_| {
                machine_uid::get().unwrap_or_else(|_| "unknown-machine".to_string())
            })
    });
        
    // Load Persistent Config
    let config = config::load_config();
    
    // Determine Password (CLI > Config > Random)
    let initial_password = args.password.clone()
        .or_else(|| config.default_passkey.clone())
        .unwrap_or_else(|| {
            let pwd = thread_rng().gen_range(100000..999999).to_string();
            pwd
        });
    
    #[cfg(target_os = "macos")]
    sys::macos_power::init_power_management();

    info!("========================================");
    info!("   SELFCONTROL AGENT v1.1");
    info!("   MACHINE ID: {}", machine_id);
    info!("   MODE:       FULL ACCESS (Integrated Service)");
    info!("========================================");

    let state = AppState::new(machine_id, initial_password);
    
    // Start Core Services on Background Threads
    core_service::start_background_services(
        state.clone(),
        args.server.clone(),
        args.port,
    );

    // Start GUI Dashboard on Main Thread
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([400.0, 350.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "SelfControl Agent",
        options,
        Box::new(|_cc| {
            Box::new(gui::dashboard::DashboardApp::new(state))
        }),
    ).map_err(|e| anyhow::anyhow!("Eframe error: {}", e))?;

    Ok(())
}
