
fn main() {

    if let Err(err) =  libcli::run() {

            eprintln!("{}", err);
        eprintln!("{}", exit_code(&err));
        std::process::exit(exit_code(&err));

    }
}

fn exit_code(err: &anyhow::Error) -> i32 {
    for cause in err.chain() {
        if let Some(pe) = cause.downcast_ref::<libprotocol::ProtocolError>() {
            return match pe {
                libprotocol::ProtocolError::Json(_) => 3,
                libprotocol::ProtocolError::Validation(_) => 3,
                libprotocol::ProtocolError::Io(_) => 2,
            };
        }
    }
    2
}

