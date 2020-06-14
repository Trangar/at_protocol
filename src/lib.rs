use serialport::{SerialPort, SerialPortSettings};
use std::time::Duration;

pub mod command;

pub struct Interface {
    port: Box<dyn SerialPort>,
}

impl Interface {
    pub fn new(port: &str) -> Result<Self, Error> {
        let port = serialport::open_with_settings(
            port,
            &SerialPortSettings {
                baud_rate: 115200,
                timeout: Duration::from_secs(30),
                ..Default::default()
            },
        )?;

        Ok(Self { port })
    }

    pub fn send<T: Command>(&mut self, command: T) -> Result<T::Output, Error> {
        const END_PHRASE: &[u8] = b"\r\nOK\r\n";
        const ERROR_PHRASE: &[u8] = b"\r\nERROR\r\n";
        let mut buffer = Vec::new();
        command
            .encode(&mut buffer)
            .map_err(|e| Error::Encode(Box::new(e)))?;

        if cfg!(debug_assertions) {
            if !buffer.ends_with(b"\r\n") {
                panic!("Command should end with \r\n");
            }
        }

        println!("> {:?}", std::str::from_utf8(&buffer).unwrap().trim());
        self.port.write_all(&buffer)?;

        buffer.clear();
        'receive_loop: loop {
            let mut buff = [0u8; 1024];
            match self.port.read(&mut buff)? {
                n if n > 0 => {
                    buffer.extend_from_slice(&buff[..n]);
                    if buffer.ends_with(END_PHRASE) {
                        buffer.drain(buffer.len() - END_PHRASE.len()..);
                        println!("< {:?}", std::str::from_utf8(&buffer).unwrap().trim());

                        break 'receive_loop;
                    }
                    if buffer.ends_with(ERROR_PHRASE) {
                        return Err(Error::Custom(String::from_utf8(buffer).unwrap()));
                    }
                }
                _ => {
                    return Err(Error::InvalidResponse(buffer));
                }
            }
        }

        command.decode(&buffer)
    }
}

pub trait Command {
    type Output;

    fn encode(&self, output: &mut impl std::io::Write) -> Result<(), Error>;
    fn decode(&self, input: &[u8]) -> Result<Self::Output, Error>;
}

#[derive(Debug)]
pub enum Error {
    Serial(serialport::Error),
    Io(std::io::Error),
    Encode(Box<Error>),
    Custom(String),
    InvalidResponse(Vec<u8>),
}

impl From<serialport::Error> for Error {
    fn from(e: serialport::Error) -> Error {
        Error::Serial(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::Io(e)
    }
}
