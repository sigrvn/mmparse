#[derive(Debug)]
pub enum DecodeErrorKind {
    InvalidFormat,
    BadData,
    Other,
}

pub struct DecodeError {
    kind: DecodeErrorKind,
    reason: String,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DecodeError {{ kind: {:?}, reason: {} }}", 
            self.kind, self.reason)
    }
}

impl From<std::io::Error> for DecodeError  {
    fn from(err: std::io::Error) -> Self {
        Self { 
            kind: DecodeErrorKind::Other,
            reason: err.to_string(),
        }
    }
}

impl From<std::str::Utf8Error> for DecodeError  {
    fn from(err: std::str::Utf8Error) -> Self {
        Self { 
            kind: DecodeErrorKind::BadData,
            reason: err.to_string(),
        }
    }
}

pub mod jpeg;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const CWD: &'static str = env!("CARGO_MANIFEST_DIR");

    #[test]
    fn it_works() {
        let mut testfile = PathBuf::from(CWD);
        testfile.push("examples/test.jpg");
        match jpeg::extract_metadata(testfile) {
            Ok(_jfif) => {},
            Err(e) => {
                eprintln!("{}", e);
                assert!(false);
            }
        }
    }
}
