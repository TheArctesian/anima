//! The Windows power boundary does not execute external programs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Mac,
    Windows,
}
#[derive(Debug, Clone)]
pub struct Platform {
    pub os: Os,
}
impl Platform {
    pub fn here() -> Self {
        Self { os: Os::Windows }
    }
}
