use rustix::cstr;

use crate::println;

fn some_tcp_server_loop() {
    rustix::thread::set_name(cstr!("tcp_server")).unwrap();

    let socket = rustix::net::socket_with(
        rustix::net::AddressFamily::INET,
        rustix::net::SocketType::STREAM,
        rustix::net::SocketFlags::CLOEXEC,
        None,
    );
    let sockfd = socket.unwrap();

    let addr = rustix::net::Ipv4Addr::new(127, 0, 0, 1);
    let addr = rustix::net::SocketAddrV4::new(addr, 0);

    rustix::net::bind_any(&sockfd, &addr.into()).unwrap();
    let addr = rustix::net::getsockname(&sockfd);
    if let Ok(rustix::net::SocketAddrAny::V4(addr)) = addr {
        println!("Listening on: {:?}:{:?}", addr.ip(), addr.port());
    }
    rustix::net::listen(&sockfd, 100).err();

    loop {
        let peer = match rustix::net::accept(&sockfd) {
            Ok(peer) => peer,
            Err(err) => {
                println!("Error: {:?}", err);
                continue;
            }
        };
        let mut buf = vec![0_u8; 1024_1024];

        // discard request
        let _ = rustix::io::read(&peer, &mut buf).unwrap();
        let response = "Hello, World!\r\n";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{response}",
            response.len()
        );
        let _ = rustix::io::write(&peer, response.as_bytes()).unwrap();
        drop(peer);
    }
}

fn some_tcp_server() {
    let thread = unsafe {
        origin::thread::create(
            |_args| {
                some_tcp_server_loop();
                None
            },
            &[None],
            origin::thread::default_stack_size(),
            origin::thread::default_guard_size(),
        )
        .unwrap()
    };
}
