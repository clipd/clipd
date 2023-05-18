macro_rules! constant {
    () => {};
    ($($i:ident => $v:expr),+$(,)?) => {
        $(
            pub const $i: &'static str = include_str!(concat!(env!("OUT_DIR"), "/", $v));
        )*
    };
}

constant! {
    BUILD_TIME => "BUILD_TIME",
    GIT_COMMIT_ID => "GIT_COMMIT_ID",
    GIT_DESCRIBE => "GIT_DESCRIBE",
    VERSION => "VERSION",
}
