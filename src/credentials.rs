use serde::Serialize;
use std::{fmt, str::FromStr};

use crate::error::KisError;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum AccountProductCode {
    DomesticStock,
}

impl AccountProductCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DomesticStock => "01",
        }
    }
}

impl fmt::Display for AccountProductCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for AccountProductCode {
    type Err = KisError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "01" => Ok(Self::DomesticStock),
            other => Err(KisError::Validation(format!(
                "{other} is not a supported account product code"
            ))),
        }
    }
}

impl Serialize for AccountProductCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
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

    pub fn with_product_code(cano: impl Into<String>, product_code: AccountProductCode) -> Self {
        Self::new(cano, product_code.as_str())
    }

    pub fn domestic_stock(cano: impl Into<String>) -> Self {
        Self::with_product_code(cano, AccountProductCode::DomesticStock)
    }

    pub(crate) fn cano(&self) -> &str {
        self.cano.expose()
    }

    pub(crate) fn product_code(&self) -> &str {
        &self.product_code
    }
}
