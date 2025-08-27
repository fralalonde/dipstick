//! A TCP Socket wrapper that reconnects automatically.

use std::fmt;
use std::io;
use std::io::Write;
use std::net::TcpStream;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};

const MIN_RECONNECT_DELAY_MS: u64 = 50;
const MAX_RECONNECT_DELAY_MS: u64 = 10_000;

/// A socket that retries
pub struct RetrySocket {
    retries: usize,
    next_try: Instant,
    addresses: Vec<SocketAddr>,
    socket: Option<TcpStream>,
}

impl fmt::Debug for RetrySocket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.next_try.fmt(f)?;
        self.socket.fmt(f)
    }
}

impl RetrySocket {
    /// Create a new socket that will retry
    pub fn new<A: ToSocketAddrs>(addresses: A) -> io::Result<Self> {
        // FIXME instead of collecting addresses early, store ToSocketAddrs as trait object
        // FIXME apparently this can not be one because of Associated Types clusterfuck (?!)
        let addresses = addresses.to_socket_addrs()?.collect();
        const INIT_DELAY: Duration = Duration::from_millis(MIN_RECONNECT_DELAY_MS);
        let next_try = Instant::now().checked_add(INIT_DELAY).expect("init delay");
        let mut socket = RetrySocket {
            retries: 0,
            next_try,
            addresses,
            socket: None,
        };

        // try early connect
        let _ = socket.flush().ok();
        Ok(socket)
    }
}

impl RetrySocket {
    fn try_connect(&mut self) -> io::Result<()> {
        if self.socket.is_none() {
            let now = Instant::now();
            if now > self.next_try {
                let addresses: &[SocketAddr] = self.addresses.as_ref();
                let socket = TcpStream::connect(addresses)?;
                socket.set_nonblocking(true)?;
                self.retries = 0;
                info!("Connected to {addresses:?}");
                self.socket = Some(socket);
            }
        }
        Ok(())
    }

    fn backoff(&mut self, e: io::Error) -> io::Error {
        self.socket = None;
        self.retries += 1;
        // double the delay with each retry
        let exp_delay = MIN_RECONNECT_DELAY_MS << self.retries;
        let max_delay = MAX_RECONNECT_DELAY_MS.min(exp_delay);
        warn!(
            "Could not connect to {:?} after {} trie(s). Backing off reconnection by {}ms. {}",
            self.addresses, self.retries, exp_delay, e
        );
        self.next_try = Instant::now() + Duration::from_millis(max_delay);
        e
    }

    fn with_socket<F, T>(&mut self, operation: F) -> io::Result<T>
    where
        F: FnOnce(&mut TcpStream) -> io::Result<T>,
    {
        if let Err(e) = self.try_connect() {
            return Err(self.backoff(e));
        }

        let opres = if let Some(ref mut socket) = self.socket {
            operation(socket)
        } else {
            // still none, quiescent
            return Err(io::Error::from(io::ErrorKind::NotConnected));
        };

        match opres {
            Ok(r) => Ok(r),
            Err(e) => Err(self.backoff(e)),
        }
    }
}

impl Write for RetrySocket {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.with_socket(|sock| sock.write(buf))
    }

    fn flush(&mut self) -> io::Result<()> {
        self.with_socket(TcpStream::flush)
    }
}
