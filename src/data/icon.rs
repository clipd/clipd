#[derive(Debug, Clone, Copy)]
pub enum Icon {
    Running,
    Paused,
}

impl Icon {
    pub fn name(&self) -> &'static str {
        match self {
            Icon::Running => "ic_01_running",
            Icon::Paused => "ic_02_paused",
        }
    }
}
