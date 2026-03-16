// KLIK stdlib - Net module

use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};

/// A TCP client connection wrapper
pub struct TcpClient {
    stream: TcpStream,
}

impl TcpClient {
    /// Connect to a remote address (e.g., "127.0.0.1:8080")
    pub fn connect(addr: &str) -> Result<Self, io::Error> {
        let stream = TcpStream::connect(addr)?;
        Ok(Self { stream })
    }

    /// Send bytes
    pub fn send(&mut self, data: &[u8]) -> Result<usize, io::Error> {
        self.stream.write(data)
    }

    /// Send string
    pub fn send_str(&mut self, data: &str) -> Result<usize, io::Error> {
        self.stream.write(data.as_bytes())
    }

    /// Receive up to `max_bytes` bytes
    pub fn recv(&mut self, max_bytes: usize) -> Result<Vec<u8>, io::Error> {
        let mut buf = vec![0u8; max_bytes];
        let n = self.stream.read(&mut buf)?;
        buf.truncate(n);
        Ok(buf)
    }

    /// Receive as string
    pub fn recv_str(&mut self, max_bytes: usize) -> Result<String, io::Error> {
        let data = self.recv(max_bytes)?;
        String::from_utf8(data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Get remote address
    pub fn remote_addr(&self) -> Result<SocketAddr, io::Error> {
        self.stream.peer_addr()
    }

    /// Get local address
    pub fn local_addr(&self) -> Result<SocketAddr, io::Error> {
        self.stream.local_addr()
    }

    /// Set read timeout in milliseconds (0 = no timeout)
    pub fn set_read_timeout(&self, millis: u64) -> Result<(), io::Error> {
        let timeout = if millis == 0 {
            None
        } else {
            Some(std::time::Duration::from_millis(millis))
        };
        self.stream.set_read_timeout(timeout)
    }

    /// Set write timeout in milliseconds (0 = no timeout)
    pub fn set_write_timeout(&self, millis: u64) -> Result<(), io::Error> {
        let timeout = if millis == 0 {
            None
        } else {
            Some(std::time::Duration::from_millis(millis))
        };
        self.stream.set_write_timeout(timeout)
    }
}

/// A TCP server
pub struct TcpServer {
    listener: TcpListener,
}

impl TcpServer {
    /// Bind to address and start listening
    pub fn bind(addr: &str) -> Result<Self, io::Error> {
        let listener = TcpListener::bind(addr)?;
        Ok(Self { listener })
    }

    /// Accept a single incoming connection
    pub fn accept(&self) -> Result<(TcpClient, SocketAddr), io::Error> {
        let (stream, addr) = self.listener.accept()?;
        Ok((TcpClient { stream }, addr))
    }

    /// Get the local address
    pub fn local_addr(&self) -> Result<SocketAddr, io::Error> {
        self.listener.local_addr()
    }

    /// Set non-blocking mode
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), io::Error> {
        self.listener.set_nonblocking(nonblocking)
    }
}

/// A UDP socket wrapper
pub struct UdpConnection {
    socket: UdpSocket,
}

impl UdpConnection {
    /// Bind to local address
    pub fn bind(addr: &str) -> Result<Self, io::Error> {
        let socket = UdpSocket::bind(addr)?;
        Ok(Self { socket })
    }

    /// Send data to a specific address
    pub fn send_to(&self, data: &[u8], addr: &str) -> Result<usize, io::Error> {
        self.socket.send_to(data, addr)
    }

    /// Receive data (returns data and sender address)
    pub fn recv_from(&self, max_bytes: usize) -> Result<(Vec<u8>, SocketAddr), io::Error> {
        let mut buf = vec![0u8; max_bytes];
        let (n, addr) = self.socket.recv_from(&mut buf)?;
        buf.truncate(n);
        Ok((buf, addr))
    }

    /// Connect to a remote address for repeated sends
    pub fn connect(&self, addr: &str) -> Result<(), io::Error> {
        self.socket.connect(addr)
    }

    /// Send data to connected peer
    pub fn send(&self, data: &[u8]) -> Result<usize, io::Error> {
        self.socket.send(data)
    }

    /// Receive data from connected peer
    pub fn recv(&self, max_bytes: usize) -> Result<Vec<u8>, io::Error> {
        let mut buf = vec![0u8; max_bytes];
        let n = self.socket.recv(&mut buf)?;
        buf.truncate(n);
        Ok(buf)
    }

    /// Get local address
    pub fn local_addr(&self) -> Result<SocketAddr, io::Error> {
        self.socket.local_addr()
    }

    /// Set non-blocking mode
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), io::Error> {
        self.socket.set_nonblocking(nonblocking)
    }
}
