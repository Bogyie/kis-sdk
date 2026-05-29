use crate::endpoint::OperationKind;

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
        self.allows_real_to_mock(method, OperationKind::Read)
    }

    pub fn allows_real_to_mock(&self, method: &str, operation_kind: OperationKind) -> bool {
        self.enabled && method == "GET" && operation_kind == OperationKind::Read
    }
}
