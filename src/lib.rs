pub mod dnstap {
    include!(concat!(env!("OUT_DIR"), "/dnstap.rs"));
}

pub mod dns;
pub mod net;
