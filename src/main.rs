mod proxy;

use cidr::Ipv6Cidr;
use getopts::Options;
use proxy::start_proxy;
use std::{collections::HashMap, env, net::SocketAddr, process::exit};
use rand::Rng;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optmulti("b", "bind", "http proxy bind addresses (can be specified multiple times)", "BIND");
    opts.optopt(
        "i",
        "ipv6-subnet",
        "IPv6 Subnet: 2a12:f8c1:55:766::/64",
        "IPv6_SUBNET",
    );
    opts.optflag("h", "help", "print this help menu");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            panic!("{}", f.to_string())
        }
    };
    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    let mut bind_addrs = matches.opt_strs("b");
    if bind_addrs.is_empty() {
        bind_addrs.push("0.0.0.0:51080".to_string());
    }
    let ipv6_subnet = matches
        .opt_str("i")
        .unwrap_or("2a12:f8c1:55:766::/64".to_string());
    run(bind_addrs, ipv6_subnet)
}

#[tokio::main]
async fn run(bind_addrs: Vec<String>, ipv6_subnet: String) {
    let ipv6_cidr = match ipv6_subnet.parse::<Ipv6Cidr>() {
        Ok(cidr) => cidr,
        Err(_) => {
            println!("invalid IPv6 subnet");
            exit(1);
        }
    };

    let mut port_to_ipv6 = HashMap::new();
    let mut rng = rand::thread_rng();

    for bind_addr in bind_addrs {
        let socket_addr: SocketAddr = match bind_addr.parse() {
            Ok(addr) => addr,
            Err(e) => {
                println!("bind address not valid: {}", e);
                continue;
            }
        };

        let port = socket_addr.port();
        let ipv6 = generate_ipv6(&ipv6_cidr, &mut rng);
        port_to_ipv6.insert(port, ipv6);

        tokio::spawn(async move {
            if let Err(e) = start_proxy(socket_addr, (ipv6, ipv6_cidr.network_length())).await {
                println!("Error starting proxy on {}: {}", bind_addr, e);
            }
        });
    }

    // Keep the main thread running
    tokio::signal::ctrl_c().await.unwrap();
    println!("Shutting down");
}

fn generate_ipv6(cidr: &Ipv6Cidr, rng: &mut impl Rng) -> std::net::Ipv6Addr {
    let mut bytes = cidr.first_address().octets();
    for i in (16 - cidr.network_length() as usize / 8)..16 {
        bytes[i] = rng.gen();
    }
    bytes.into()
}