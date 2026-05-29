#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FallbackPolicy {
    enabled: bool,
}

impl FallbackPolicy {
    pub fn disabled() -> Self {
        Self { enabled: false }
    }

    pub fn real_to_mock_reads() -> Self {
        Self { enabled: true }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn allows_real_to_mock_read(&self, method: &str) -> bool {
        self.enabled && method == "GET"
    }
}
