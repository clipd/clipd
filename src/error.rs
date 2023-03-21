pub trait ExpectWithTracing<T> {
    fn expectx<S: AsRef<str>>(self, msg: S) -> T;
}

impl<T, E: std::fmt::Debug> ExpectWithTracing<T> for Result<T, E> {
    fn expectx<S: AsRef<str>>(self, msg: S) -> T {
        match self {
            Ok(o) => {
                log::trace!("{}", msg.as_ref());
                o
            }
            Err(e) => {
                let msg = format!("{} failed with: {:?}", msg.as_ref(), e);
                log::error!("{}", msg);
                panic!("{}", msg)
            }
        }
    }
}

impl<T> ExpectWithTracing<T> for Option<T> {
    fn expectx<S: AsRef<str>>(self, name: S) -> T {
        match self {
            Some(o) => o,
            None => {
                let msg = format!("Expect '{}' but not found", name.as_ref());
                log::error!("{}", msg);
                panic!("{}", msg)
            }
        }
    }
}
