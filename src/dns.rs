// pub struct FauxDns {}

// impl Dns for FauxDns {
//     /// The type returned when we have an error
//     type Error = RustixTCPError;

//     async fn get_host_by_name(
//         &self,
//         _host: &str,
//         _addr_type: embedded_nal_async::AddrType,
//     ) -> Result<embedded_nal_async::IpAddr, Self::Error> {
//         Ok(embedded_nal_async::IpAddr::V4(
//             embedded_nal_async::Ipv4Addr::new(127, 0, 0, 1),
//         ))
//     }

//     async fn get_host_by_address(
//         &self,
//         _addr: embedded_nal_async::IpAddr,
//     ) -> Result<embedded_nal_async::heapless::String<256>, Self::Error> {
//         Ok(embedded_nal_async::heapless::String::from("localhost"))
//     }
// }

use core::net::SocketAddr;

use alloc::string::String;
use dns_protocol::{Flags, Message, Question, ResourceRecord, ResourceType};
use embedded_nal_async::AddrType;
use rustix::{
    fd::OwnedFd,
    net::{AddressFamily, RecvFlags, SendFlags},
};


#[derive(thiserror::Error, Debug)]
pub enum LookupError {
    #[error("protocol error: {0}")]
    DnsProtocolError(dns_protocol::Error),

    #[error("unexpected response {0}")]
    UnexpectedResponse(String),

    #[error("not found")]
    NotFound,

    #[error("socket errno: {0}")]
    SocketError(rustix::io::Errno),
}

type Result<T> = core::result::Result<T, LookupError>;

pub struct DnsClient {
    server: SocketAddr,
}

impl DnsClient {
    pub fn new_ipv4(octets: [u8; 4], port: u16) -> Self {
        let ip = core::net::Ipv4Addr::new(octets[0], octets[1], octets[2], octets[3]);
        let server = SocketAddr::new(ip.into(), port);

        Self { server }
    }

    fn bind(&self) -> Result<OwnedFd> {
        let domain = match &self.server {
            SocketAddr::V4(_) => AddressFamily::INET,
            SocketAddr::V6(_) => AddressFamily::INET6,
        };

        let socket = rustix::net::socket(
            domain,
            rustix::net::SocketType::DGRAM,
            Some(rustix::net::ipproto::UDP),
        )
        .map_err(LookupError::SocketError)?;

        Ok(socket)
    }

    pub fn nslookup(&self, name: &str, addr_type: AddrType) -> Result<core::net::IpAddr> {
        let mut questions = [Question::new(name, ResourceType::A, 1)];
        let message = Message::new(
            0xFEE7,
            Flags::standard_query(),
            &mut questions,
            &mut [],
            &mut [],
            &mut [],
        );

        let mut buffer = vec![0; message.space_needed()];
        message
            .write(&mut buffer)
            .map_err(LookupError::DnsProtocolError)?;

        let socket = self.bind()?;
        rustix::net::sendto(&socket, &buffer, SendFlags::empty(), &self.server)
            .map_err(LookupError::SocketError)?;

        let mut buffer = vec![0; 1024];
        let (len, _) = rustix::net::recvfrom(&socket, &mut buffer, RecvFlags::empty())
            .map_err(LookupError::SocketError)?;

        let mut answers = [ResourceRecord::default(); 16];
        let mut authority = [ResourceRecord::default(); 16];
        let mut additional = [ResourceRecord::default(); 16];
        let message = Message::read(
            &buffer[..len],
            &mut questions,
            &mut answers,
            &mut authority,
            &mut additional,
        )
        .map_err(LookupError::DnsProtocolError)?;

        fn process_answers(
            answers: &[ResourceRecord],
            addr_type: &AddrType,
        ) -> Result<core::net::IpAddr> {
            for answer in answers {
                match (answer.data().len(), &addr_type) {
                    (4, AddrType::IPv4 | AddrType::Either) => {
                        let mut ip = [0u8; 4];
                        ip.copy_from_slice(answer.data());
                        let ip = core::net::Ipv4Addr::from(ip);
                        return Ok(core::net::IpAddr::V4(ip));
                    }
                    (16, AddrType::IPv6 | AddrType::Either) => {
                        let mut ip = [0u8; 16];
                        ip.copy_from_slice(answer.data());
                        let ip = core::net::Ipv6Addr::from(ip);
                        return Ok(core::net::IpAddr::V6(ip));
                    }
                    _ => {}
                }
            }
            Err(LookupError::NotFound)
        }

        process_answers(message.answers(), &addr_type)
    }
}

impl embedded_nal_async::Dns for DnsClient {
    type Error = LookupError;

    async fn get_host_by_name(
        &self,
        host: &str,
        addr_type: embedded_nal_async::AddrType,
    ) -> core::result::Result<embedded_nal_async::IpAddr, Self::Error> {
        

        self.nslookup(host, addr_type).map(|res| match res {
            core::net::IpAddr::V4(ip) => embedded_nal_async::IpAddr::V4(ip.octets().into()),
            core::net::IpAddr::V6(ip) => embedded_nal_async::IpAddr::V6(ip.octets().into()),
        })
    }

    async fn get_host_by_address(
        &self,
        _addr: embedded_nal_async::IpAddr,
    ) -> core::result::Result<embedded_nal_async::heapless::String<256>, Self::Error> {
        Ok(embedded_nal_async::heapless::String::from("unimplemented"))
    }
}
