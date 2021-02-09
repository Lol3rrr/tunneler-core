/// Describes a simple external Address consisting of an IP and Port
#[derive(Clone)]
pub struct Destination {
    ip: String,
    port: u32,
    formatted: String,
}

impl Destination {
    /// Creates a new Destination from the given data
    pub fn new(ip: String, port: u32) -> Self {
        let formatted_ip = format!("{}:{}", ip, port);

        Destination {
            ip,
            port,
            formatted: formatted_ip,
        }
    }

    /// Tries to connect to the described Destination and returns
    /// the TCP-Stream that was established
    pub async fn connect(&self) -> std::io::Result<tokio::net::TcpStream> {
        let stream = tokio::net::TcpStream::connect(&self.formatted).await?;

        Ok(stream)
    }

    /// Returns the full address to connect to
    pub fn get_full_address(&self) -> &str {
        &self.formatted
    }

    /// Returns the Raw-IP of the Destination
    pub fn get_ip(&self) -> &str {
        &self.ip
    }
    /// Returns the Port of the Destination
    pub fn get_port(&self) -> u32 {
        self.port
    }
}

#[test]
fn new_dest_ip() {
    let dest = Destination::new("localhost".to_owned(), 123);
    assert_eq!("localhost", dest.get_ip());
}

#[test]
fn new_dest_port() {
    let dest = Destination::new("localhost".to_owned(), 123);
    assert_eq!(123, dest.get_port());
}

#[test]
fn new_dest_formatted() {
    let dest = Destination::new("localhost".to_owned(), 123);
    assert_eq!("localhost:123", dest.get_full_address());
}
