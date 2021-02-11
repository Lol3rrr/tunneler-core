use tokio::net::TcpStream;

#[derive(Debug)]
pub struct Connection {
    stream: TcpStream,
}

impl Connection {
    pub fn new(tcp_con: TcpStream) -> Connection {
        Connection { stream: tcp_con }
    }

    pub async fn read_raw(&self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.stream.readable().await {
            Ok(_) => {}
            Err(e) => {
                return Err(e);
            }
        };

        self.stream.try_read(buf)
    }

    pub async fn write_raw(&self, buf: &[u8]) -> std::io::Result<usize> {
        match self.stream.writable().await {
            Ok(_) => {}
            Err(e) => {
                return Err(e);
            }
        };

        self.stream.try_write(buf)
    }

    pub async fn write_total(&self, data: &[u8], length: usize) -> std::io::Result<()> {
        let mut offset = 0;
        let mut left_to_send = length;

        while left_to_send > 0 {
            match self.write_raw(&data[offset..offset + left_to_send]).await {
                Ok(0) => {
                    return Err(std::io::Error::from(std::io::ErrorKind::ConnectionReset));
                }
                Ok(n) => {
                    offset += n;
                    left_to_send -= n;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e);
                }
            };
        }

        Ok(())
    }

    pub async fn read_total(&self, data: &mut [u8], length: usize) -> std::io::Result<()> {
        let mut offset = 0;
        let mut left_to_read = length;

        while left_to_read > 0 {
            match self
                .read_raw(&mut data[offset..offset + left_to_read])
                .await
            {
                Ok(0) => {
                    return Err(std::io::Error::from(std::io::ErrorKind::ConnectionReset));
                }
                Ok(n) => {
                    offset += n;
                    left_to_read -= n;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e);
                }
            };
        }

        Ok(())
    }
}
