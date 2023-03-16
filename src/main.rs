use clap::{Parser, ValueEnum};
use socket2::{Domain, InterfaceIndexOrAddress, Protocol, Socket, Type};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use tokio::{io::AsyncBufReadExt, net::UdpSocket};

#[derive(Clone, Debug, PartialEq)]
pub enum Either<T, U> {
    Left(T),
    Right(U),
}

#[derive(Clone, Debug, PartialEq, ValueEnum)]
pub enum Mode {
    Listen,
    Talk,
}

impl From<Either<u32, Ipv4Addr>> for InterfaceIndexOrAddress {
    fn from(either: Either<u32, Ipv4Addr>) -> Self {
        match either {
            Either::Left(index) => InterfaceIndexOrAddress::Index(index),
            Either::Right(addr) => InterfaceIndexOrAddress::Address(addr.into()),
        }
    }
}

fn parse_interface(
    s: &str,
) -> Result<Either<u32, Ipv4Addr>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    if let Ok(index) = s.parse::<u32>() {
        Ok(Either::Left(index))
    } else {
        Ok(Either::Right(s.parse()?))
    }
}

#[derive(Parser)]
struct Config {
    #[arg(short = 'i', value_parser = parse_interface, default_value = "0")]
    iface: Either<u32, Ipv4Addr>,

    #[arg(short = 'a', default_value = "0.0.0.0")]
    mc_addr: IpAddr,

    #[arg(short = 'p')]
    mc_port: u16,

    #[arg(short = 'm', value_enum)]
    mode: Mode,
}

fn mc_socket(mc_addr: SocketAddr, iface: InterfaceIndexOrAddress) -> std::io::Result<UdpSocket> {
    let mc_ip = mc_addr.ip();
    assert!(mc_ip.is_multicast());
    let synsocket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    synsocket.set_nonblocking(true)?;
    synsocket.set_reuse_address(true)?;
    synsocket.bind(&mc_addr.into())?;

    let socket = UdpSocket::from_std(synsocket.into())?;

    match (mc_ip, iface) {
        (IpAddr::V4(mc_ip), InterfaceIndexOrAddress::Address(if_ip)) => {
            socket.join_multicast_v4(mc_ip, if_ip)?;
        }
        (IpAddr::V6(mc_ip), InterfaceIndexOrAddress::Index(if_id)) => {
            socket.join_multicast_v6(&mc_ip, if_id)?;
        }
        _ => panic!("Invalid combination of multicast IP address and interface"),
    }

    Ok(socket)
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cfg = Config::parse();

    let multi = match cfg.mc_addr {
        IpAddr::V4(ip) => SocketAddrV4::new(ip, cfg.mc_port).into(),
        IpAddr::V6(ip) => SocketAddrV6::new(ip, cfg.mc_port, 0, 0).into(),
    };

    let socket = mc_socket(multi, cfg.iface.into()).unwrap();

    match cfg.mode {
        Mode::Listen => {
            let mut buf = [0; 1024];
            while let Ok((size, addr)) = socket.recv_from(&mut buf).await {
                println!("Received {} bytes from {:?}", size, addr);
            }
        }
        Mode::Talk => {
            let i = tokio::io::stdin();
            let mut lines = tokio::io::BufReader::new(i).lines();
            while let Some(line) = lines.next_line().await.unwrap() {
                socket.send_to(line.as_bytes(), multi).await.unwrap();
            }
        }
    }
}
