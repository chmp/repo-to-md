use anyhow::bail;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AtLeastOne<T>(Vec<T>);

impl<T> AtLeastOne<T> {
    pub fn head(&self) -> &T {
        self.first()
            .unwrap_or_else(|| unreachable!("AtLeastOne contains at least one item"))
    }

    pub fn tail(&self) -> &[T] {
        self.get(1..)
            .unwrap_or_else(|| unreachable!("AtLeastOne contains at least one item"))
    }
}

impl<T> TryFrom<Vec<T>> for AtLeastOne<T> {
    type Error = anyhow::Error;

    fn try_from(value: Vec<T>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            bail!("AtLeastOne requires at least one item");
        }
        Ok(Self(value))
    }
}

impl<T> From<AtLeastOne<T>> for Vec<T> {
    fn from(value: AtLeastOne<T>) -> Self {
        value.0
    }
}

impl<T> From<T> for AtLeastOne<T> {
    fn from(value: T) -> Self {
        Self(vec![value])
    }
}

impl<T> std::ops::Deref for AtLeastOne<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::convert::AsRef<Vec<T>> for AtLeastOne<T> {
    fn as_ref(&self) -> &Vec<T> {
        &self.0
    }
}
