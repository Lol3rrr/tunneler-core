mod ip;
pub use ip::{DeserializeIPError, DetailsIP};

/// The Details about a single Connection
#[derive(Debug, PartialEq)]
pub struct Details {
    ip: DetailsIP,
}

#[derive(Debug, PartialEq)]
pub enum DeserializeDetailsError {
    IPError(DeserializeIPError),
}

impl From<DeserializeIPError> for DeserializeDetailsError {
    fn from(e: DeserializeIPError) -> Self {
        Self::IPError(e)
    }
}

impl Details {
    pub(crate) fn new(ip: DetailsIP) -> Self {
        Self { ip }
    }

    pub(crate) fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();

        result.extend(self.ip.serialize());

        result
    }

    pub(crate) fn deserialize(data: &mut Vec<u8>) -> Result<Details, DeserializeDetailsError> {
        let ip = DetailsIP::deserialize(data)?;

        Ok(Details { ip })
    }
}
