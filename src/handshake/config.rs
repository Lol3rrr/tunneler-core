use std::convert::TryInto;

use crate::PROTOCOL_VERSION;

/// The Configuration for Connecting to a Server, this contains all the needed Data for
/// establishing a Connection, like the desired Port
#[derive(Debug, PartialEq, Clone)]
pub struct Config {
    port: u16,
    /// The Version number of the Protocol, although I never exepct this to exceed 255(8-bit) it's
    /// better to be save with this than regret it later on
    prot_version: u16,
}

#[derive(Debug, PartialEq)]
pub enum ConfigError {
    InvalidPort,
}

impl Config {
    /// Creates a new Config Instance for the given Port
    pub fn new(port: u16) -> Self {
        Self {
            port,
            prot_version: PROTOCOL_VERSION,
        }
    }

    /// The Port of the Configuration
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The Protocol Version defined in the Config
    pub fn protocol_version(&self) -> u16 {
        self.prot_version
    }

    /// Converts the Config into its Byte representation to be transmitted over the network when
    /// connecting
    pub fn to_bytes(&self) -> [u8; 4] {
        let mut result = [0; 4];

        result[0..2].copy_from_slice(&self.port.to_be_bytes());
        result[2..4].copy_from_slice(&self.prot_version.to_be_bytes());

        result
    }

    /// Converts the Raw-Bytes back into a Config Instance, returns an Error if the given Bytes are
    /// not correctly representing a Config
    pub fn from_bytes(raw: &[u8]) -> Result<Self, ConfigError> {
        if raw.len() < 2 {
            return Err(ConfigError::InvalidPort);
        }

        let port_bytes = &raw[0..2];
        let port = u16::from_be_bytes(port_bytes.try_into().unwrap());

        let prot_version = match raw.len() {
            x if x < 4 => 0,
            _ => {
                let prot_bytes = &raw[2..4];
                let prot = u16::from_be_bytes(prot_bytes.try_into().unwrap());
                prot
            }
        };

        Ok(Self { port, prot_version })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_bytes() {
        let conf = Config {
            port: 13,
            prot_version: 1,
        };

        let port_bytes = 13_u16.to_be_bytes();
        let prot_version_bytes = 1_u16.to_be_bytes();
        let expected = [
            port_bytes[0],
            port_bytes[1],
            prot_version_bytes[0],
            prot_version_bytes[1],
        ];

        let result = conf.to_bytes();

        assert_eq!(expected, result);
    }

    #[test]
    fn from_bytes_port() {
        let input = 13_u16.to_be_bytes();

        let expected = Ok(Config {
            port: 13,
            prot_version: 0,
        });

        let result = Config::from_bytes(&input);

        assert_eq!(expected, result);
    }
    #[test]
    fn from_bytes_port_version() {
        let mut input = [0; 4];
        input[0..2].copy_from_slice(&13_u16.to_be_bytes());
        input[2..4].copy_from_slice(&1_u16.to_be_bytes());

        let expected = Ok(Config {
            port: 13,
            prot_version: 1,
        });

        let result = Config::from_bytes(&input);

        assert_eq!(expected, result);
    }

    #[test]
    fn missing_port() {
        let input = &[];

        let expected = Err(ConfigError::InvalidPort);

        let result = Config::from_bytes(input);

        assert_eq!(expected, result);
    }
}
