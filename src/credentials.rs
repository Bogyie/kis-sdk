use std::fmt;

#[derive(Clone, Eq, PartialEq)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretString([REDACTED])")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppCredentials {
    app_key: SecretString,
    app_secret: SecretString,
}

impl AppCredentials {
    pub fn new(app_key: impl Into<String>, app_secret: impl Into<String>) -> Self {
        Self {
            app_key: SecretString::new(app_key),
            app_secret: SecretString::new(app_secret),
        }
    }

    pub(crate) fn app_key(&self) -> &str {
        self.app_key.expose()
    }

    pub(crate) fn app_secret(&self) -> &str {
        self.app_secret.expose()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Account {
    cano: SecretString,
    product_code: String,
}

impl Account {
    pub fn new(cano: impl Into<String>, product_code: impl Into<String>) -> Self {
        Self {
            cano: SecretString::new(cano),
            product_code: product_code.into(),
        }
    }

    pub(crate) fn cano(&self) -> &str {
        self.cano.expose()
    }

    pub(crate) fn product_code(&self) -> &str {
        &self.product_code
    }
}
