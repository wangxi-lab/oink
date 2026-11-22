//! Restrict device traffic to directly connected private IPv4 networks.
use std::{
    cell::RefCell,
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::{Duration, Instant},
};

type Subnet = (Ipv4Addr, u8);

thread_local! {
    static INTERFACES: RefCell<Option<(Instant, Vec<Subnet>)>> = const { RefCell::new(None) };
}

fn private_ipv4(address: Ipv4Addr) -> bool {
    address.is_private() || address.is_link_local()
}

fn ipv4(address: IpAddr) -> Option<Ipv4Addr> {
    match address {
        IpAddr::V4(address) => Some(address),
        IpAddr::V6(address) => address.to_ipv4_mapped(),
    }
}

fn mask(prefix: u8) -> Option<u32> {
    // Ignore overly broad or malformed interface prefixes.
    (8..=32).contains(&prefix).then(|| u32::MAX << (32 - prefix))
}

fn permits(address: Ipv4Addr, interfaces: &[Subnet]) -> bool {
    if address.is_loopback() {
        return true;
    }
    if !private_ipv4(address) {
        return false;
    }
    let address = u32::from(address);
    interfaces.iter().any(|&(local, prefix)| {
        if !private_ipv4(local) {
            return false;
        }
        let Some(mask) = mask(prefix) else { return false };
        let network = u32::from(local) & mask;
        address & mask == network && (prefix >= 31 || (address != network && address != (network | !mask)))
    })
}

fn allows_peer(address: SocketAddr, interfaces: &[Subnet]) -> bool {
    address.port() != 0 && ipv4(address.ip()).is_some_and(|address| permits(address, interfaces))
}

pub(super) fn allows(address: SocketAddr) -> bool {
    let Some(ip) = ipv4(address.ip()) else { return false };
    if !private_ipv4(ip) {
        return allows_peer(address, &[]);
    }
    with_subnets(|subnets| allows_peer(address, subnets))
}

pub(super) fn broadcasts(bind: IpAddr) -> Vec<Ipv4Addr> {
    with_subnets(|subnets| {
        subnets
            .iter()
            .filter_map(|&(ip, prefix)| {
                if prefix >= 31 || !private_ipv4(ip) || (!bind.is_unspecified() && bind != IpAddr::V4(ip)) {
                    return None;
                }
                let broadcast = Ipv4Addr::from(u32::from(ip) | !mask(prefix)?);
                private_ipv4(broadcast).then_some(broadcast)
            })
            .collect()
    })
}

fn with_subnets<T>(use_subnets: impl FnOnce(&[Subnet]) -> T) -> T {
    INTERFACES.with_borrow_mut(|cached| {
        let (updated, subnets) = cached.get_or_insert_with(|| (Instant::now(), subnets()));
        if updated.elapsed() >= Duration::from_secs(1) {
            *subnets = self::subnets();
            *updated = Instant::now();
        }
        use_subnets(subnets)
    })
}

fn subnets() -> Vec<Subnet> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|interface| match interface.addr {
            if_addrs::IfAddr::V4(address) => Some((address.ip, address.prefixlen)),
            if_addrs::IfAddr::V6(_) => None,
        })
        .collect()
}

pub(super) fn require(address: SocketAddr) -> io::Result<()> {
    if allows(address) {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "peer is outside the local IPv4 network",
        ))
    }
}

#[cfg(test)]
mod lan_policy_tests {
    #[test]
    fn peer_policy_rejects_public_off_link_and_reserved_addresses() {
        let interfaces = [("192.168.20.5", 24), ("8.8.8.4", 24)];
        let interfaces = interfaces.map(|(ip, prefix)| (ip.parse().unwrap(), prefix));
        for (address, permitted) in [
            ("127.0.0.1:47470", true),
            ("192.168.20.9:47470", true),
            ("192.168.20.0:47470", false),
            ("192.168.20.255:47470", false),
            ("192.168.21.9:47470", false),
            ("8.8.8.8:47470", false),
            ("224.0.0.251:47470", false),
            ("192.168.20.9:0", false),
            ("[::ffff:8.8.8.8]:47470", false),
            ("[::ffff:192.168.20.9]:47470", true),
            ("[2001:4860:4860::8888]:47470", false),
        ] {
            assert_eq!(
                super::allows_peer(address.parse().unwrap(), &interfaces),
                permitted,
                "{address}"
            );
        }
    }
}
