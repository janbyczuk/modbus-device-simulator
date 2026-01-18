//
// server.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Nov 26 2022
//
use crate::device::DeviceManager;

use std::{
    io,
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
};
use anyhow::Result;
use futures::future::Future;

use tokio_modbus::{
    prelude::*,
    server::{self, Service, NewService},
};

/// Handles spawning new service handlers for modbus clients
struct ServiceSpawner {
    device_manager: Arc<DeviceManager>,
}

impl NewService for ServiceSpawner {
    type Request = Request;
    type Response = Response;
    type Error = io::Error;
    type Instance = DeviceService;

    fn new_service(&self) -> std::io::Result<Self::Instance> {
        Ok(DeviceService::new(self.device_manager.clone()))
    }
}

/// Handles dispatching requests to the virtual device manager
struct DeviceService {
    device_manager: Arc<DeviceManager>,
}

impl DeviceService {
    pub fn new(device_manager: Arc<DeviceManager>) -> Self {
        Self { device_manager }
    }
}

impl Service for DeviceService {
    type Request = Request;
    type Response = Response;
    type Error = io::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + Sync>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let device_manager = self.device_manager.clone();

        Box::pin(async move {
            // Extract slave_id from request (if available in the protocol layer)
            // For now, we'll use a default slave_id of 1
            // In a proper implementation, this would come from the Modbus request header
            let slave_id = 1u8;

            let res = match req {
                Request::ReadInputRegisters(addr, cnt) => {
                    let regs = device_manager
                        .read_input_registers(slave_id, addr, cnt)
                        .await
                        .unwrap_or(vec![]);
                    Response::ReadInputRegisters(regs)
                }
                Request::ReadDiscreteInputs(addr, cnt) => {
                    let inputs = device_manager
                        .read_discrete_inputs(slave_id, addr, cnt)
                        .await
                        .unwrap_or(vec![]);
                    Response::ReadDiscreteInputs(inputs)
                }
                Request::ReadCoils(addr, cnt) => {
                    let coils = device_manager
                        .read_coils(slave_id, addr, cnt)
                        .await
                        .unwrap_or(vec![]);
                    Response::ReadCoils(coils)
                }
                Request::ReadHoldingRegisters(addr, cnt) => {
                    let regs = device_manager
                        .read_holding_registers(slave_id, addr, cnt)
                        .await
                        .unwrap_or(vec![]);
                    Response::ReadHoldingRegisters(regs)
                }
                Request::WriteMultipleCoils(addr, coils) => {
                    let (address, written) = device_manager
                        .write_coils(slave_id, addr, coils)
                        .await
                        .unwrap_or((addr, 0));
                    Response::WriteMultipleCoils(address, written)
                }
                Request::WriteSingleCoil(address, value) => {
                    let (address, _) = device_manager
                        .write_coils(slave_id, address, vec![value])
                        .await
                        .unwrap_or((address, 0));
                    Response::WriteSingleCoil(address, value)
                }
                Request::WriteMultipleRegisters(addr, values) => {
                    let (address, written) = device_manager
                        .write_holding_registers(slave_id, addr, values)
                        .await
                        .unwrap_or((addr, 0));
                    Response::WriteMultipleRegisters(address, written)
                }
                _ => unimplemented!(),
            };

            Ok(res)
        })
    }
}

/// Main Modbus TCP server task
pub async fn run(sock_addr: SocketAddr, device_manager: DeviceManager) -> Result<()> {
    let spawner = ServiceSpawner {
        device_manager: Arc::new(device_manager),
    };

    // Create a modbus tcp server and start with the service spawner
    let modbus_server = server::tcp::Server::new(sock_addr);
    modbus_server.serve(spawner).await?;

    Ok(())
}
