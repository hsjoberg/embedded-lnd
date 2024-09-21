#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

mod bidi_stream;
mod event_subscription;
mod lnd_client;

pub use bidi_stream::BidiStreamBuilder;
pub use event_subscription::EventSubscriptionBuilder;
pub use lnd_client::LndClient;

#[cfg(test)]
mod tests;
