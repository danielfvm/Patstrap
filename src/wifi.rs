use async_trait::async_trait;
use mdns_sd::{ResolvedService, ScopedIp, ServiceDaemon, ServiceEvent};
use std::collections::HashSet;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;

use crate::device::{Connection, ConnectionCtx, Device};

const DNS_WIFI_SERVICE: &'static str = "patstrap";

pub struct WifiConnection {
    info: ResolvedService,
}

#[async_trait]
impl Connection for WifiConnection {
    type Ctx = ConnectionCtx;

    async fn read(&self, ctx: &mut Self::Ctx) -> anyhow::Result<Vec<u8>> {
        let ConnectionCtx::Wifi { reader, .. } = ctx else {
            return Err(anyhow::Error::msg("Context error"));
        };

        //let mut buffer = vec![];
        //reader.read(&mut buffer).await?;

        let mut b = [0; 32];
        let len = reader.read(&mut b).await?;
        let buffer = b[..len].to_vec();

        Ok(buffer)
    }

    async fn write(&self, ctx: &mut Self::Ctx, data: &[u8]) -> anyhow::Result<usize> {
        let ConnectionCtx::Wifi { writer, .. } = ctx else {
            return Err(anyhow::Error::msg("Context error"));
        };

        let res = writer.write(data).await.map_err(Into::into);

        let _ = writer.flush().await;

        res
    }

    async fn disconnect(&self, ctx: Self::Ctx) -> anyhow::Result<()> {
        let ConnectionCtx::Wifi { reader, writer } = ctx else {
            return Err(anyhow::Error::msg("Context error"));
        };

        let mut tcp = reader.reunite(writer)?;
        tcp.shutdown().await?;

        Ok(())
    }

    async fn connect(&self) -> anyhow::Result<Self::Ctx> {
        let addr = SocketAddr::new(self.ip().to_ip_addr(), self.info.port);
        let tcp = TcpStream::connect(addr).await?;
        let (reader, writer) = tcp.into_split();

        Ok(ConnectionCtx::Wifi { reader, writer })
    }

    fn name(&self) -> String {
        self.info.host.clone()
    }

    fn addr(&self) -> String {
        format!("{}:{}", self.ip(), self.info.port)
    }

    fn id(&self) -> String {
        self.info.fullname.clone()
    }
}

impl WifiConnection {
    pub fn new(info: ResolvedService) -> Self {
        Self { info }
    }

    pub fn ip(&self) -> ScopedIp {
        self.info.get_addresses().iter().nth(0).unwrap().clone()
    }

    pub async fn discover(
        devices: Arc<Mutex<Vec<Device>>>,
    ) -> Result<(), Box<dyn Error>> {
        // Create daemon
        let mdns = ServiceDaemon::new()?;

        // Start browsing for a service type
        let service_type = "_http._tcp.local.";
        let receiver = mdns.browse(service_type)?;

        println!("Browsing for services...");

        let mut services_found = HashSet::new();

        loop {
            tokio::select! {
                result = tokio::task::spawn_blocking({
                    let receiver = receiver.clone();
                    move || receiver.recv_timeout(Duration::from_millis(500))
                }) => {
                    if let Ok(Ok(event)) = result {
                        match event {
                            ServiceEvent::ServiceResolved(info) => {
                                println!("Resolved service:");
                                println!("  Name: {}", info.get_fullname());
                                println!("  Host: {}", info.get_hostname());
                                println!("  Port: {}", info.get_port());

                                if !services_found.contains(&info.fullname) {
                                    services_found.insert(info.fullname.clone());
                                } else {
                                    continue;
                                }

                                let name = info.get_hostname();
                                if !name.contains(DNS_WIFI_SERVICE) {
                                    continue;
                                }

                                if let Ok(ref mut lock) = devices.try_lock() {
                                    lock.push(Device::new(WifiConnection::new(*info)));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

/*use log::info;
use log::warn;
use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::collections::HashSet;
use std::error::Error;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_util::sync::CancellationToken;

use crate::device::CommandClient;
use crate::device::{Connectable, ConnectableDevice, ConnectionStatus, Device};

const DNS_WIFI_SERVICE: &'static str = "patstrap";

pub struct WifiDevice {
    inner: ConnectableDevice<SocketAddr>,
    addr: SocketAddr,
    name: String,
}

impl WifiDevice {
    pub fn new(name: String, addr: SocketAddr) -> Device {
        let inner = ConnectableDevice::new(Arc::new(addr), WifiDevice::setup);

        Device::new(Self { name, addr, inner })
    }

    async fn setup(
        addr: Arc<SocketAddr>,
        token: CancellationToken,
        tx: Sender<Vec<u8>>,
        mut rx: Receiver<Vec<u8>>,
    ) -> anyhow::Result<()> {
        let tcp = TcpStream::connect(*addr).await?;
        let (mut reader, mut writer) = tcp.into_split();
        let mut buffer = [0u8; 1024];

        info!("Connection to {} established", addr);

        loop {
            tokio::select! {
                res = reader.read(&mut buffer) => {
                    match res {
                        Ok(0) => break,
                        Ok(len) => {
                            println!("Result: {:?} {:?} {}", &buffer[..len], CommandClient::from_bytes(&buffer[..len]), len);
                            tx.send(buffer[..len].to_vec()).await?;
                        }
                        Err(err) => {
                            warn!("Error reading from WifiDevice: {err}");
                            break;
                        }
                    }
                }
                Some(data) = rx.recv() => {
                    println!("Sending: {:?}", data);
                    writer.write_all(&data).await?;
                }
                _ = token.cancelled() => {
                    info!("Disconnected!");
                    break;
                }
            }
        }

        /*if let Ok(mut tcp) = reader.reunite(writer) {
            tcp.shutdown().await?;
        }*/

        Ok(())
    }

    pub async fn discover(
        devices: Arc<Mutex<Vec<Device>>>,
        cancel: CancellationToken,
    ) -> Result<(), Box<dyn Error>> {
        // Create daemon
        let mdns = ServiceDaemon::new()?;

        // Start browsing for a service type
        let service_type = "_http._tcp.local.";
        let receiver = mdns.browse(service_type)?;

        println!("Browsing for services...");

        let mut services_found = HashSet::new();

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    println!("Shutting down mDNS task...");
                    break;
                }
                result = tokio::task::spawn_blocking({
                    let receiver = receiver.clone();
                    move || receiver.recv_timeout(Duration::from_millis(500))
                }) => {
                    if let Ok(Ok(event)) = result {
                        match event {
                            ServiceEvent::ServiceResolved(info) => {
                                println!("Resolved service:");
                                println!("  Name: {}", info.get_fullname());
                                println!("  Host: {}", info.get_hostname());
                                println!("  Port: {}", info.get_port());

                                let addr = info
                                    .get_addresses()
                                    .iter()
                                    .nth(0)
                                    .unwrap()
                                    .clone();

                                /*for addr in info.get_addresses() {
                                    println!("  Address: {}", addr);
                                }

                                for property in info.get_properties().iter() {
                                    println!("{} {:?}", property.key(), property.val());
                                }*/

                                if !services_found.contains(&info.fullname) {
                                    services_found.insert(info.fullname.clone());
                                } else {
                                    continue;
                                }

                                let name = info.get_hostname();
                                if !name.contains(DNS_WIFI_SERVICE) {
                                    continue;
                                }

                                if let Ok(ref mut lock) = devices.try_lock() {
                                    lock.push(WifiDevice::new(name.to_string(), SocketAddr::new(addr.to_ip_addr(), info.port)));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl Connectable for WifiDevice {
    fn status(&self) -> ConnectionStatus {
        self.inner.status()
    }

    fn disconnect(&mut self) {
        info!("Disconnecting {}", self.id());
        self.inner.disconnect();
    }

    fn connect(&mut self) {
        info!("Connecting {}", self.id());
        self.inner.connect();
    }

    fn id(&self) -> String {
        self.addr.to_string()
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

impl Read for WifiDevice {
    fn read(&mut self, mut buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Write for WifiDevice {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}*/
