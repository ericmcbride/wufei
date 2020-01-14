use std::io::{Error, ErrorKind};

/// Since Command returns stdout or stderr attrs instead of actual errors, we need a helper
/// function to generate custom errors when dealing with Command.
pub fn generate_err(err_msg: String) -> Result<(), Box<dyn::std::error::Error>> {
    Err(Box::new(Error::new(ErrorKind::Other, err_msg)))
}
