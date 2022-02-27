use std::fmt::Debug;

/// The Strategy used to determine if a given Port is valid
/// according to the Strategy and its parameters
pub enum Strategy {
    /// Only the given Port is accepted and
    /// every other one is rejected
    Single(u16),
    /// Accepts the Ports in the Vector
    Multiple(Vec<u16>),
    /// Accepts all the Ports in the Range
    Ranged(std::ops::Range<u16>),
    /// Accepts all the Ports to be open
    Always,
    /// This allows you to specify whatever strategy you need
    Custom(Box<dyn Send + Sync + Fn(u16) -> bool>),
}

impl Debug for Strategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Single(port) => f.debug_tuple("Single").field(&port).finish(),
            Self::Multiple(ports) => f.debug_tuple("Multiple").field(&ports).finish(),
            Self::Ranged(range) => f.debug_tuple("Ranged").field(&range).finish(),
            Self::Always => f.debug_tuple("Always").finish(),
            Self::Custom(_) => f.debug_tuple("Custom").finish(),
        }
    }
}

impl Strategy {
    /// Checks if the given Port conforms to the
    /// Strategy that is being used
    pub fn contains_port(&self, port: u16) -> bool {
        match self {
            Self::Single(tmp) => *tmp == port,
            Self::Multiple(tmp) => tmp.contains(&port),
            Self::Ranged(range) => range.contains(&port),
            Self::Always => true,
            Self::Custom(func) => func(port),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_valid() {
        let strat = Strategy::Single(13);
        assert_eq!(true, strat.contains_port(13));
    }
    #[test]
    fn single_invalid() {
        let strat = Strategy::Single(13);
        assert_eq!(false, strat.contains_port(52));
    }

    #[test]
    fn multiple_valid() {
        let strat = Strategy::Multiple(vec![10, 13, 52]);
        assert_eq!(true, strat.contains_port(13));
    }
    #[test]
    fn multiple_invalid() {
        let strat = Strategy::Multiple(vec![13, 53]);
        assert_eq!(false, strat.contains_port(52));
    }

    #[test]
    fn ranged_range_valid() {
        let strat = Strategy::Ranged(10..15);
        assert_eq!(true, strat.contains_port(13));
    }
    #[test]
    fn ranged_range_invalid() {
        let strat = Strategy::Ranged(10..51);
        assert_eq!(false, strat.contains_port(52));
    }

    #[test]
    fn always_valid() {
        let strat = Strategy::Always;
        assert_eq!(true, strat.contains_port(1312));
    }
}
