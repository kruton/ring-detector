pub mod dnstap {
    include!(concat!(env!("OUT_DIR"), "/dnstap.rs"));
}
pub mod bridge;
pub mod dns;
pub mod mqtt;
pub mod net;
