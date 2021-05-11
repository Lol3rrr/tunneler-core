/// The Strategy used to determine if a given Port is valid
/// according to the Strategy and its parameters
#[derive(Debug, PartialEq)]
pub enum Strategy {
    /// Only the given Port is accepted and
    /// every other one is rejected
    Single(u16),
    /// Accepts the Ports in the Vector
    Multiple(Vec<u16>),
    /// Accepts all the Ports in the Range
    Dynamic(Option<std::ops::Range<u16>>),
}

impl Strategy {
    /// Checks if the given Port conforms to the
    /// Strategy that is being used
    pub fn contains_port(&self, port: u16) -> bool {
        match self {
            Self::Single(tmp) => *tmp == port,
            Self::Multiple(tmp) => tmp.contains(&port),
            Self::Dynamic(tmp) => match tmp {
                Some(range) => range.contains(&port),
                None => true,
            },
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
    fn dynamic_valid() {
        let strat = Strategy::Dynamic(None);
        assert_eq!(true, strat.contains_port(13));
    }
    #[test]
    fn dynamic_range_valid() {
        let strat = Strategy::Dynamic(Some(10..15));
        assert_eq!(true, strat.contains_port(13));
    }
    #[test]
    fn dynamic_range_invalid() {
        let strat = Strategy::Dynamic(Some(10..51));
        assert_eq!(false, strat.contains_port(52));
    }
}
