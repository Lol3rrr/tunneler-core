/// An IPV4 or IPV6 Address
///
/// The Address is represented in its Octets format
#[derive(Debug, PartialEq)]
pub enum DetailsIP {
    /// An IPV4 Address
    IPV4([u8; 4]),
    /// An IPV6 Address
    IPV6([u8; 16]),
}

impl From<std::net::SocketAddr> for DetailsIP {
    fn from(ip_v: std::net::SocketAddr) -> Self {
        match ip_v {
            std::net::SocketAddr::V4(addr) => {
                let ip = addr.ip();
                DetailsIP::IPV4(ip.octets())
            }
            std::net::SocketAddr::V6(addr) => {
                let ip = addr.ip();
                DetailsIP::IPV6(ip.octets())
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum DeserializeIPError {
    UnknownID,
    TooSmall,
}

impl DetailsIP {
    /// Serializes the IP address to be send over the Network
    pub(crate) fn serialize(&self) -> Vec<u8> {
        match self {
            DetailsIP::IPV4(data) => {
                let mut result = vec![0; 5];
                result[0] = 4;
                result[1..5].copy_from_slice(data);

                result
            }
            DetailsIP::IPV6(data) => {
                let mut result = vec![0; 17];
                result[0] = 6;
                result[1..17].copy_from_slice(data);

                result
            }
        }
    }

    /// Deserializes the given Byte-Sequence into an IP-Address
    /// and removes all the bytes that correspond to it from the
    /// Sequence
    pub(crate) fn deserialize(data: &mut Vec<u8>) -> Result<Self, DeserializeIPError> {
        match data.get(0) {
            Some(4) => {
                if data.len() < 5 {
                    return Err(DeserializeIPError::TooSmall);
                }

                let mut result = [0; 4];
                result.copy_from_slice(&data[1..5]);
                data.drain(0..5);
                Ok(DetailsIP::IPV4(result))
            }
            Some(6) => {
                if data.len() < 17 {
                    return Err(DeserializeIPError::TooSmall);
                }

                let mut result = [0; 16];
                result.copy_from_slice(&data[1..17]);
                data.drain(0..17);
                Ok(DetailsIP::IPV6(result))
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
        let ip = DetailsIP::IPV4([1, 2, 3, 4]);
        let expected = vec![4, 1, 2, 3, 4];
        assert_eq!(expected, ip.serialize());
    }
    #[test]
    fn deserialize_v4() {
        let mut data = vec![4, 1, 2, 3, 4];
        let expected = Ok(DetailsIP::IPV4([1, 2, 3, 4]));
        assert_eq!(expected, DetailsIP::deserialize(&mut data));
        assert_eq!(0, data.len());
    }

    #[test]
    fn serialize_v6() {
        let ip = DetailsIP::IPV6([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
        let expected = vec![6, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        assert_eq!(expected, ip.serialize());
    }
    #[test]
    fn deserialize_v6() {
        let mut data = vec![6, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let expected = Ok(DetailsIP::IPV6([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
        ]));
        assert_eq!(expected, DetailsIP::deserialize(&mut data));
        assert_eq!(0, data.len());
    }
}
