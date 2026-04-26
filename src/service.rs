use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddrV4},
    sync::{Arc, Mutex},
    thread::sleep,
    time::Duration,
};

use tokio::net::UdpSocket;
use tokio_util::sync::CancellationToken;
use vrchat_osc::{
    Error, VRChatOSC,
    models::OscRootNode,
    rosc::{self, OscPacket},
};

use crate::{config::Config, device::Device};

pub struct OSCService {
    osc: Arc<VRChatOSC>,
}

struct Data {
    prev: f32,
    curr: f32,
    change: f32,
}

impl OSCService {
    pub async fn new(
        devices: Arc<Mutex<Vec<Device>>>,
        token: CancellationToken,
        app_status: Arc<Mutex<bool>>,
    ) -> Result<(), Error> {
        let vrchat_osc = VRChatOSC::new(None).await?;

        let root_node = OscRootNode::new().with_avatar();
        let data = Arc::new(Mutex::new(HashMap::new()));
        let data2 = data.clone();

        //        println!("{}", vrchat_osc.get_osc_ip());

        vrchat_osc
            .register("patstrap", root_node, move |packet| {
                if let OscPacket::Message(msg) = packet {
                    // Indicate we have connection to VRChat
                    *app_status.lock().unwrap() = true;

                    if !msg.addr.starts_with("/avatar/parameters/pat_") {
                        return;
                    }

                    let addr = msg.addr.to_string();

                    let mut data = data2.lock().unwrap();

                    let entry = data.entry(addr).or_insert(Data {
                        prev: 0.0,
                        curr: 0.0,
                        change: 0.0,
                    });

                    entry.prev = entry.curr;
                    entry.curr = msg.args[0].clone().float().unwrap();
                    entry.change += (entry.curr - entry.prev) * 10.0;
                }
            })
            .await?;

        while !token.is_cancelled() {
            sleep(Duration::from_millis(50));

            let Ok(mut devices) = devices.try_lock() else {
                continue;
            };

            for (id, d) in data.lock().unwrap().iter_mut() {
                if d.change > 0.05 {
                    let change = d.change.clamp(0.1, 1.0);
                    for device in devices.iter_mut() {
                        //device.haptic_osc(id, 255);
                        device.haptic_osc(id, (change.floor() * 255.0) as u8, 50);
                    }
                }
                d.change /= 2.0;
            }
        }

        Ok(())
    }
}

impl Drop for OSCService {
    fn drop(&mut self) {
        let osc = self.osc.clone();
        tokio::task::spawn(async move {
            let _ = osc.shutdown().await;
        });
    }
}

pub struct OSCUnity {}

impl OSCUnity {
    pub async fn new(
        devices: Arc<Mutex<Vec<Device>>>,
        app_status: Arc<Mutex<bool>>,
    ) -> anyhow::Result<()> {
        let port = Config::instance().unity_osc_port;
        let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)).await?;
        let osc_local_addr = socket.local_addr()?; // Get the actual address it bound to.

        log::info!("OSCUnity listening on {}:{}", osc_local_addr, port);

        // Spawn a task to handle incoming OSC packets.
        let _osc_handle = tokio::spawn(async move {
            let mut buf = [0; 65535]; // Buffer for receiving OSC packets.
            loop {
                // Wait to receive data on the socket.
                match socket.recv_from(&mut buf).await {
                    Ok((len, addr)) => {
                        // Decode the received UDP data into an OSC packet.
                        if let Ok((_, packet)) = rosc::decoder::decode_udp(&buf[..len]) {
                            if let OscPacket::Message(msg) = packet {
                                *app_status.lock().unwrap() = msg.addr != "/disconnect";

                                if msg.addr == "/haptic" {
                                    let Ok(mut devices) = devices.try_lock() else {
                                        continue;
                                    };

                                    let Some(haptic_name) =
                                        msg.args.get(0).map(|i| i.clone().string()).flatten()
                                    else {
                                        continue;
                                    };

                                    let Some(haptic_strength) =
                                        msg.args.get(1).map(|i| i.clone().float()).flatten()
                                    else {
                                        continue;
                                    };

                                    let Some(haptic_duration) =
                                        msg.args.get(2).map(|i| i.clone().float()).flatten()
                                    else {
                                        continue;
                                    };

                                    let haptic_strength = (haptic_strength * 255.0) as u8;
                                    let haptic_duration = (haptic_duration * 1000.0) as u16;

                                    for device in devices.iter_mut() {
                                        device.haptic_osc(
                                            &haptic_name,
                                            haptic_strength,
                                            haptic_duration,
                                        );
                                    }
                                }
                            }
                        } else {
                            log::debug!("Failed to decode OSC packet from {}", addr);
                        }
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::ConnectionReset
                            || e.kind() == std::io::ErrorKind::BrokenPipe
                        {
                            log::warn!(
                                "Socket connection error ({}). Task for {:?} might need to be restarted or interface is down.",
                                e,
                                socket.local_addr().ok()
                            );
                            break;
                        } else {
                            log::warn!(
                                "Failed to receive data on OSC socket {:?}: {}",
                                socket.local_addr().ok(),
                                e
                            );
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        continue;
                    }
                }
            }
        });
        Ok(())
    }
}
