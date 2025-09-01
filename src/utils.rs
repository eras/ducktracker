use serde::Deserialize;

#[derive(Debug)]
pub struct CommaSeparatedVec<T>(pub Vec<T>);

impl<T> CommaSeparatedVec<T> {
    pub fn new() -> Self {
        CommaSeparatedVec(Vec::new())
    }
}

impl<'de, T> Deserialize<'de> for CommaSeparatedVec<T>
where
    T: Clone + std::str::FromStr + std::fmt::Display,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s.is_empty() {
            return Ok(CommaSeparatedVec(Vec::new()));
        }
        let vec = s
            .split(',')
            .map(|s| s.trim().parse::<T>())
            .collect::<Result<Vec<T>, _>>()
            .map_err(|_err| serde::de::Error::custom("Failed to parse comma separated list"))?;
        Ok(CommaSeparatedVec(vec))
    }
}

impl<T> std::fmt::Display for CommaSeparatedVec<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for field in self.0.iter() {
            if !first {
                write!(f, ",")?;
            }
            first = false;
            write!(f, "{}", field)?;
        }
        Ok(())
    }
}
