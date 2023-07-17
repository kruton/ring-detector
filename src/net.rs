/*
 * Copyright 2023 Kenny Root
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

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
