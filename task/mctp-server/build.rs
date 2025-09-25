fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    idol::server::build_server_support(
        "../../idl/mctp.idol",
        "server_stub.rs",
        idol::server::ServerStyle::InOrder,
    )?;
    // uart irq
    build_util::build_notifications()?;

    Ok(())
}
