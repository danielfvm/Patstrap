//
// CommandClient/Server: {
//  cmd_type: u8, 
//  cmd_len: u8,
//  ...arbitrary data...
// }
//

pub struct Reader<'a> {
    data: &'a [u8],
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    fn u8(&mut self) -> Option<u8> {
        let (&b, rest) = self.data.split_first()?;
        self.data = rest;
        Some(b)
    }

    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes([self.u8()?, self.u8()?]))
    }

    fn take(&mut self, split: usize) -> Option<&[u8]> {
        let (left, right) = self.data.split_at(split);
        self.data = right;
        Some(left)
    }

    fn rest(&mut self) -> Option<&[u8]> {
        Some(std::mem::replace(&mut self.data, &[]))
    }
}

#[repr(u16)]
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum CommandServer {
    Haptic {
        channel: u8,   // Selected haptic
        strength: u8,  // Strength between 0 (none) and 255 (max)
        duration: u16, // Length in ms
    },
}

impl CommandServer {
    pub fn as_bytes(&self) -> Vec<u8> {
        /*match *self {
            Self::Haptic {
                channel,
                strength,
                duration,
            } => vec![ 3, 6, channel, strength]
                .into_iter()
                .chain(duration.to_le_bytes())
                .collect::<Vec<u8>>(),
        }*/

        match *self {
            Self::Haptic {
                channel,
                strength,
                duration,
            } => {
               let hex_string = format!("{:02X}{:02X}{:04X}\x0A", channel, strength, duration);
               hex_string.into_bytes()
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum CommandClient {
    Connected,
    Disconnect,
    Info { channel: u8, name: String },
    Battery { level: u8 },
    Invalid,
}

impl CommandClient {
    pub fn from_bytes(r: &mut Reader<'_>) -> Option<Self> {
        let cmd_type = r.u8()?;
        let cmd_len = r.u8()? as usize;

        match cmd_type {
            0 => Some(Self::Invalid),
            1 if cmd_len >= 4 => Some(Self::Info {
                channel: r.u8()?,
                name: String::from_utf8(r.take(cmd_len - 3)?.to_vec()).unwrap(),
            }),
            2 if cmd_len == 3 => Some(Self::Battery { level: r.u8()? }),
            _ => None, // Invalid tag!
        }
    }
}
