use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// The Details about a single Connection
#[derive(Debug, PartialEq)]
pub struct Details {
    ip: IpAddr,
}

#[derive(Debug)]
pub enum DeserializeDetailsError {
    DeserializeError(Box<dyn std::fmt::Debug>),
}

impl Details {
    pub(crate) fn new(ip: IpAddr) -> Self {
        Self { ip }
    }

    pub(crate) fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();

        result.extend(self.ip.serialize());

        result
    }

    pub(crate) fn deserialize(data: &mut Vec<u8>) -> Result<Details, DeserializeDetailsError> {
        let ip = match IpAddr::deserialize(data) {
            Ok(i) => i,
            Err(e) => return Err(DeserializeDetailsError::DeserializeError(Box::new(e))),
        };

        Ok(Details { ip })
    }

    /// The IP address of the User
    pub fn ip(&self) -> &IpAddr {
        &self.ip
    }
}

trait SerializeDetails: Sized {
    type DeserializeError: std::fmt::Debug;

    fn serialize(&self) -> Vec<u8>;
    fn deserialize(buffer: &mut Vec<u8>) -> Result<Self, Self::DeserializeError>;
}

#[derive(Debug)]
enum DeserializeIPError {
    UnknownID,
    Incomplete,
}
impl SerializeDetails for IpAddr {
    type DeserializeError = DeserializeIPError;

    fn serialize(&self) -> Vec<u8> {
        match self {
            IpAddr::V4(ip4) => {
                let octets = ip4.octets();

                let mut result = vec![0; 5];
                result[0] = 4;
                result[1..5].copy_from_slice(&octets);

                result
            }
            IpAddr::V6(ip6) => {
                let octets = ip6.octets();

                let mut result = vec![0; 17];
                result[0] = 6;
                result[1..17].copy_from_slice(&octets);

                result
            }
        }
    }
    fn deserialize(buffer: &mut Vec<u8>) -> Result<Self, Self::DeserializeError> {
        match buffer.get(0) {
            Some(4) => {
                if buffer.len() < 5 {
                    return Err(DeserializeIPError::Incomplete);
                }

                let mut result = [0; 4];
                result.copy_from_slice(&buffer[1..5]);
                buffer.drain(0..5);
                Ok(IpAddr::V4(Ipv4Addr::from(result)))
            }
            Some(6) => {
                if buffer.len() < 17 {
                    return Err(DeserializeIPError::Incomplete);
                }

                let mut result = [0; 16];
                result.copy_from_slice(&buffer[1..17]);
                buffer.drain(0..17);
                Ok(IpAddr::V6(Ipv6Addr::from(result)))
            }
            _ => Err(DeserializeIPError::UnknownID),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_v4() {
        let ip = IpAddr::V4(Ipv4Addr::from([1, 2, 3, 4]));
        let expected = vec![4, 1, 2, 3, 4];
        assert_eq!(expected, ip.serialize());
    }
    #[test]
    fn deserialize_v4() {
        let mut data = vec![4, 1, 2, 3, 4];
        let expected = IpAddr::V4(Ipv4Addr::from([1, 2, 3, 4]));

        let result = IpAddr::deserialize(&mut data);
        assert_eq!(true, result.is_ok());
        assert_eq!(expected, result.unwrap());
        assert_eq!(0, data.len());
    }

    #[test]
    fn serialize_v6() {
        let ip = IpAddr::V6(Ipv6Addr::from([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
        ]));
        let expected = vec![6, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        assert_eq!(expected, ip.serialize());
    }
    #[test]
    fn deserialize_v6() {
        let mut data = vec![6, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let expected = IpAddr::V6(Ipv6Addr::from([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
        ]));

        let result = IpAddr::deserialize(&mut data);
        assert_eq!(true, result.is_ok());
        assert_eq!(expected, result.unwrap());
        assert_eq!(0, data.len());
    }
}
