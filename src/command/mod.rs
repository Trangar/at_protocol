mod wifi_mode;

pub use self::wifi_mode::*;

macro_rules! simple_command {
    (
        $(#[$outer:meta])*
        $name:ident => $blob:expr
    ) => {
        $(#[$outer])*
        pub struct $name;

        impl crate::Command for $name {
            type Output = bool;

            fn encode(&self, buffer: &mut impl std::io::Write) -> Result<(), crate::Error> {
                buffer.write_all($blob).map_err(Into::into)
            }

            fn decode(&self, buffer: &[u8]) -> Result<bool, crate::Error> {
                Ok(buffer == $blob)
            }
        }
    };
}

simple_command!(
    /// Test if AT system works correctly
    Test => b"AT\r\n"
);

simple_command!(
    /// Reset the module
    ///
    /// Note: Often your serial connection will be reset after running this command. To be safe, re-create your serial connection.
    Restart => b"AT+RST\r\n"
);

simple_command!(
    /// Disconnect from the current AP
    DisconnectFromAp => b"AT+CWQAP\r\n"
);

pub struct GetVersion;

impl crate::Command for GetVersion {
    type Output = String;

    fn encode(&self, buffer: &mut impl std::io::Write) -> Result<(), crate::Error> {
        buffer.write_all(b"AT+GMR\r\n").map_err(Into::into)
    }

    fn decode(&self, buffer: &[u8]) -> Result<String, crate::Error> {
        let str = std::str::from_utf8(buffer).unwrap();
        let newline_pos = str.bytes().position(|b| b == b'\n').unwrap();
        let str = (&str[newline_pos..]).trim();
        Ok(str.to_owned())
    }
}
