use async_trait::async_trait;
use btleplug::api::{
    Central, CentralEvent, Characteristic, Manager as _, Peripheral as _, PeripheralProperties,
    ScanFilter,
};
use btleplug::platform::{Manager, Peripheral, PeripheralId};
use futures_util::StreamExt;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use uuid::{Uuid, uuid};

use crate::PatstrapError;
use crate::device::{Connection, ConnectionCtx, Device};

// Bluetooth service characteristic used by patstrap
const CHARACTERISTIC_UUID: Uuid = uuid!("49535343-1e4d-4bd9-ba61-23c647249616");
const BLE_SERVICE_UUID: Uuid = uuid!("49535343-fe7d-4ae5-8fa9-9fafd205e455");

pub struct BleConnection {
    peripheral: Peripheral,
    properties: PeripheralProperties,
}

#[async_trait]
impl Connection for BleConnection {
    type Ctx = ConnectionCtx;

    async fn read(&self, ctx: &mut Self::Ctx) -> anyhow::Result<Vec<u8>> {
        let ConnectionCtx::Ble { notifications, .. } = ctx else {
            return Err(anyhow::Error::msg("Context error"));
        };

        notifications
            .next()
            .await
            .map(|notification| notification.value)
            .ok_or(anyhow::Error::msg("Error in write"))
    }

    async fn write(&self, ctx: &mut Self::Ctx, data: &[u8]) -> anyhow::Result<usize> {
        let ConnectionCtx::Ble { characteristic, .. } = ctx else {
            return Err(anyhow::Error::msg("Context error"));
        };

        self.peripheral
            .write(
                &characteristic,
                &data,
                btleplug::api::WriteType::WithoutResponse,
            )
            .await
            .map(|_| data.len())
            .map_err(|err| err.into())
    }

    async fn disconnect(&self, _: Self::Ctx) -> anyhow::Result<()> {
        Ok(())
    }

    async fn connect(&self) -> anyhow::Result<Self::Ctx> {
        if !self.peripheral.is_connected().await? {
            self.peripheral.connect().await?;
        }

        // discover services and characteristics
        self.peripheral.discover_services().await?;

        let chars = self.peripheral.characteristics();
        let characteristic = chars
            .iter()
            .find(|c| c.uuid == CHARACTERISTIC_UUID)
            .ok_or(std::io::Error::other("Characteristic not found"))?;

        self.peripheral.subscribe(&characteristic).await?;

        Ok(ConnectionCtx::Ble {
            characteristic: characteristic.clone(),
            notifications: self.peripheral.notifications().await?,
        })
    }

    fn name(&self) -> String {
        self.properties
            .local_name
            .clone()
            .unwrap_or("???".to_owned())
    }

    fn addr(&self) -> String {
        self.properties.address.to_string()
    }

    fn id(&self) -> String {
        self.peripheral.id().to_string()
    }
}

impl BleConnection {
    pub fn new(peripheral: Peripheral, properties: PeripheralProperties) -> Self {
        Self {
            peripheral,
            properties,
        }
    }

    pub async fn discover(devices: Arc<Mutex<Vec<Device>>>) -> Result<(), Box<dyn Error>> {
        let Ok(manager) = Manager::new().await else {
            return Err(Box::new(PatstrapError::NoBluetoothAdapter));
        };

        // get the first bluetooth adapter
        let adapters = manager.adapters().await?;
        let Some(central) = adapters.into_iter().nth(0) else {
            return Err(Box::new(PatstrapError::NoBluetoothAdapter));
        };

        let mut events = central.events().await?;

        // start scanning for devices
        central.start_scan(ScanFilter::default()).await?;

        let mut peripherals: HashMap<PeripheralId, Peripheral> = HashMap::new();

        while let Some(event) = events.next().await {
            match event {
                CentralEvent::DeviceDiscovered(id) => {
                    let peripheral = central.peripheral(&id).await?;
                    let Some(properties) = peripheral.properties().await? else {
                        continue;
                    };

                    let name = properties.local_name.unwrap_or_default();
                    let addr = properties.address.to_string();
                    println!("DeviceDiscovered: {:?} {}", addr, name);

                    peripherals.insert(peripheral.id(), peripheral);
                }
                CentralEvent::StateUpdate(state) => {
                    println!("AdapterStatusUpdate {:?}", state);
                }
                CentralEvent::DeviceConnected(id) => {
                    println!("DeviceConnected: {:?}", id);
                }
                CentralEvent::DeviceDisconnected(id) => {
                    println!("DeviceDisconnected: {:?}", id);
                    if let Ok(ref mut lock) = devices.try_lock() {
                        lock.retain(|device| device.id() != id.to_string());
                    }
                }
                CentralEvent::ServicesAdvertisement { id, services } => {
                    if services
                        .into_iter()
                        .any(|service| service == BLE_SERVICE_UUID)
                    {
                        if let Some(peripheral) = peripherals.remove(&id) {
                            let Some(properties) = peripheral.properties().await? else {
                                break;
                            };

                            if let Ok(ref mut lock) = devices.try_lock() {
                                lock.push(Device::new(BleConnection::new(peripheral, properties)));
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}
