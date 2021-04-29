use crate::{Command, Error};

/// Get the current wifi mode of the module.
pub struct GetWifiMode;

impl Command for GetWifiMode {
    type Output = WifiMode;

    fn encode(&self, buffer: &mut impl std::io::Write) -> Result<(), Error> {
        buffer.write_all(b"AT+CWMODE?\r\n").map_err(Into::into)
    }

    fn decode(&self, buffer: &[u8]) -> Result<WifiMode, Error> {
        // Response is:
        // "AT+CWMODE?
        //  +CWMODE:1\r\n"
        // so we look for the first ':', then take the next char

        let invalid_response = || Error::Custom(format!("Invalid response: {:?}", buffer));

        let index = buffer
            .iter()
            .position(|b| *b == b':')
            .ok_or_else(invalid_response)?;
        let next = buffer.get(index + 1).ok_or_else(invalid_response)?;

        match next {
            b'1' => Ok(WifiMode::StationMode),
            b'2' => Ok(WifiMode::ApMode),
            b'3' => Ok(WifiMode::ApStationMode),
            x => Err(Error::Custom(format!(
                "Unknown wifi mode: {}, expected {}, {} or {}",
                x, b'1', b'2', b'3'
            ))),
        }
    }
}
/// Set the current wifi mode of the module.
pub struct SetWifiMode(pub WifiMode);

impl Command for SetWifiMode {
    type Output = ();

    fn encode(&self, buffer: &mut impl std::io::Write) -> Result<(), Error> {
        write!(buffer, "AT+CWMODE={}\r\n", self.0 as u8)?;
        Ok(())
    }

    fn decode(&self, _buffer: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum WifiMode {
    StationMode = 1,
    ApMode = 2,
    ApStationMode = 3,
}

/// List all available access points in range.
///
/// Note: the chip needs to be in `WifiMode::StationMode`. You can check the current mode with [GetWifiMode] and you can change this with [SetWifiMode].
pub struct ListAp;

impl Command for ListAp {
    type Output = Vec<AccessPoint>;

    fn encode(&self, buffer: &mut impl std::io::Write) -> Result<(), Error> {
        buffer.write_all(b"AT+CWLAP\r\n").map_err(Into::into)
    }

    fn decode(&self, buffer: &[u8]) -> Result<Vec<AccessPoint>, Error> {
        // Response is in this format:
        // "AT+CWLAP\r\n"
        // "+CWLAP:(<ecn>,<ssid>,<rssi>,<mac>)\r\n"
        // we make the assumption this is valid UTF8 just to make the parsing easier
        let str = std::str::from_utf8(buffer).unwrap();
        let mut result = Vec::new();
        for line in str.lines().filter(|l| l.starts_with("+CWLAP:(")) {
            let open_bracket = line.bytes().position(|b| b == b'(').unwrap();
            let line = &line[open_bracket + 1..];

            let (ecn, line) = try_get_string_until(line, b',')?;
            let (ssid, line) = try_get_string_until(line, b',')?;
            let (rssi, line) = try_get_string_until(line, b',')?;
            let (mac, line) = try_get_string_until(line, b',')?;
            let (channel, _line) = try_get_string_until(line, b')')?;

            let ecn: u8 = match ecn.parse() {
                Ok(ecn) => ecn,
                Err(e) => {
                    return Err(Error::Custom(format!(
                        "Invalid ECN value {:?}: {:?}",
                        ecn, e
                    )))
                }
            };

            let ecn = match ecn {
                0 => ECN::Open,
                1 => ECN::WEP,
                2 => ECN::WPA_PSK,
                3 => ECN::WPA2_PSK,
                4 => ECN::WPA_WPA2_PSK,
                x => ECN::Unknown(x),
            };
            let rssi: i16 = match rssi.parse() {
                Ok(rssi) => rssi,
                Err(e) => {
                    return Err(Error::Custom(format!(
                        "Invalid RSSI value: {:?}: {:?}",
                        rssi, e
                    )))
                }
            };
            let channel: u8 = match channel.parse() {
                Ok(channel) => channel,
                Err(e) => {
                    return Err(Error::Custom(format!(
                        "Invalid channel value: {:?}: {:?}",
                        channel, e
                    )))
                }
            };

            result.push(AccessPoint {
                ecn,
                ssid: ssid.to_owned(),
                rssi,
                mac: mac.to_owned(),
                channel,
            })
        }

        Ok(result)
    }
}

fn try_get_string_until(str: &str, find: u8) -> Result<(&str, &str), Error> {
    let mut in_quotes = false;
    for (index, byte) in str.bytes().enumerate() {
        if byte == b'"' {
            in_quotes = !in_quotes;
        }
        if byte == find && !in_quotes {
            let mut lhs = &str[..index];
            if lhs.len() >= 2 && lhs.starts_with('"') && lhs.ends_with('"') {
                lhs = &lhs[1..lhs.len() - 1];
            }
            let rhs = &str[index + 1..];
            return Ok((lhs, rhs));
        }
    }
    Err(Error::Custom(format!(
        "Could not find character {:?} in string {:?}",
        find, str
    )))
}

#[derive(Debug)]
pub struct AccessPoint {
    pub ecn: ECN,
    pub ssid: String,
    pub rssi: i16,
    pub mac: String, // Should this be a byte array?
    pub channel: u8,
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
pub enum ECN {
    Open,
    WEP,
    WPA_PSK,
    WPA2_PSK,
    WPA_WPA2_PSK,
    Unknown(u8),
}

pub struct ConnectToAp<'a> {
    pub ssid: &'a str,
    pub password: &'a str,
}

impl<'a> Command for ConnectToAp<'a> {
    type Output = ();

    fn encode(&self, output: &mut impl std::io::Write) -> Result<(), Error> {
        write!(output, "AT+CWJAP={:?},{:?}\r\n", self.ssid, self.password)?;
        Ok(())
    }

    fn decode(&self, _input: &[u8]) -> Result<Self::Output, Error> {
        Ok(())
    }
}

pub struct GetConnectedAp;

impl Command for GetConnectedAp {
    type Output = Option<String>;

    fn encode(&self, output: &mut impl std::io::Write) -> Result<(), Error> {
        output.write_all(b"AT+CWJAP?\r\n").map_err(Into::into)
    }

    fn decode(&self, input: &[u8]) -> Result<Self::Output, Error> {
        // response: "AT+CWJAP?\r\n+CWJAP:\"<SSID>\",\"0c:d6:bd:0e:50:10\",8,-49,0,0,0,0"
        // or: "AT+CWJAP?\r\nNo AP"
        let input = std::str::from_utf8(input).unwrap();
        let line = input.lines().nth(1).unwrap().trim();
        if line == "No AP" {
            return Ok(None);
        }
        let colon = line.bytes().position(|b| b == b':').unwrap();
        let comma = line.bytes().position(|b| b == b',').unwrap();
        let mut name = &line[colon + 1..comma];
        if name.len() >= 2 && name.starts_with('"') && name.ends_with('"') {
            name = &name[1..name.len() - 1];
        }

        Ok(Some(name.to_owned()))
    }
}
