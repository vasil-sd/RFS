pub fn one() -> u32 { 1 }

pub struct Alpha;

impl Alpha {
    pub fn make() -> Self {
        Alpha
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn one_is_one() {
        assert_eq!(super::one(), 1);
    }
}
