use edge_dhcp::{
    io::DEFAULT_SERVER_PORT,
    server::{Server, ServerOptions},
    Ipv4Addr, Options, Packet,
};
use embassy_net::{
    udp::{PacketMetadata, UdpSocket},
    IpAddress, Ipv4Address,
};
use esp_println::println;
use esp_wifi::wifi::{WifiApDevice, WifiDevice};

type Stack = embassy_net::Stack<WifiDevice<'static, WifiApDevice>>;

#[embassy_executor::task]
pub async fn run_dhcp_server(stack: &'static Stack) {
    let ip = Ipv4Addr::new(192, 168, 4, 1);

    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut rx_buffer = [0; 4096];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_buffer = [0; 4096];

    let mut socket = UdpSocket::new(
        stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );
    socket.bind(DEFAULT_SERVER_PORT).unwrap();

    let mut buf = [0; 1500];
    let mut server_options = ServerOptions::new(ip, None);
    let mut server = Server::<16>::new(ip);

    loop {
        let (len, mut remote) = match socket.recv_from(&mut buf).await {
            Ok(x) => x,
            Err(e) => {
                println!("DHCP receive error {e:?}");
                continue;
            }
        };
        let packet = &buf[..len];

        let request = match Packet::decode(packet) {
            Ok(request) => request,
            Err(e) => {
                println!("DHCP decode error {e:?}");
                continue;
            }
        };

        let mut opt_buf = Options::buf();

        if let Some(reply) = server.handle_request(&mut opt_buf, &mut server_options, &request) {
            // we might add ipv6 later
            #[allow(irrefutable_let_patterns)]
            if let IpAddress::Ipv4(addr_ipv4) = remote.addr {
                if request.broadcast || addr_ipv4.is_unspecified() {
                    remote.addr = IpAddress::Ipv4(Ipv4Address::BROADCAST)
                }
            }

            if let Err(e) = socket
                .send_to(reply.encode(&mut buf).unwrap(), remote)
                .await
            {
                println!("DHCP send error {e:?}");
                continue;
            }
        }
    }
}
