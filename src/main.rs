//
// This program is a script-able a modbus device server
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Nov 26 2022
//

use mbsim::{
    cli::Args,
    server,
    device::DeviceManager,
    config::Config,
};
use clap::Parser;
use anyhow::{Result, Context};

use std::{
    fs,
    net::SocketAddr
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();

    let sock_addr: SocketAddr = format!("{}:{}", args.ip, args.port).parse()
        .with_context(|| format!("Invalid host/port"))?;

    println!("Starting Modbus server on: {}", sock_addr);

    // Load YAML configuration
    let yaml = fs::read_to_string(&args.config)
        .with_context(|| format!("Failed to read config file: {}", args.config))?;

    let config = Config::from_yaml(&yaml)
        .with_context(|| "Failed to parse YAML configuration")?;

    // Create the device manager from configuration
    let device_manager = DeviceManager::from_config(config);

    // Start the server task
    tokio::spawn(server::run(sock_addr, device_manager));
    // Wait for user exit
    tokio::signal::ctrl_c().await?;

    Ok(())
}
