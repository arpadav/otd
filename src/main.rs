//! A simple file server for sharing files over the local network.
//!
//! Author: aav
/// The main entry point of OTD
fn main() -> std::io::Result<()> {
    smol::block_on(async {
        // --------------------------------------------------
        // initialize logging
        // --------------------------------------------------
        otd::init_logging().await;
        // --------------------------------------------------
        // run
        // --------------------------------------------------
        let server = otd::Server::new().await;
        server.run().await
    })
}
