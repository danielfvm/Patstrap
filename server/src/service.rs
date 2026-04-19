use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::sleep,
    time::Duration,
};

use tokio_util::sync::CancellationToken;
use vrchat_osc::{Error, VRChatOSC, models::OscRootNode, rosc::OscPacket};

use crate::device::Device;

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
                        device.haptic_osc(id, (change.floor() * 255.0) as u8);
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
