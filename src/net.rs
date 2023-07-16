use anyhow::{anyhow, Error};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    result::Result,
};

type IpError = Error;

pub fn parse_octets(value: &[u8]) -> Result<IpAddr, IpError> {
    match value.len() {
        4 => {
            let addr: [u8; 4] = value.try_into().unwrap();
            Ok(IpAddr::V4(Ipv4Addr::from(addr)))
        }
        16 => {
            let addr: [u8; 16] = value.try_into().unwrap();
            Ok(IpAddr::V6(Ipv6Addr::from(addr)))
        }
        _ => Err(anyhow!("unexpected address length")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_4_octets_pass() {
        let addr = parse_octets(&[127, 0, 0, 1]);
        assert_eq!("127.0.0.1", addr.unwrap().to_string());
    }

    #[test]
    fn from_16_octets_pass() {
        let addr = parse_octets(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        assert_eq!("::1", addr.unwrap().to_string());
    }

    #[test]
    fn from_0_octets_failure() {
        let addr = parse_octets(&[]);
        assert!(addr.is_err(), "0 octets should not be parseable");
    }

    #[test]
    fn from_6_octets_failure() {
        let addr = parse_octets(&[127, 0, 0, 0, 0, 0, 1]);
        assert!(addr.is_err(), "6 octets should not be parseable");
    }
}
