/// The rename target, declared exactly once in the fixture tree.
pub fn target_fn(x: u32) -> u32 {
    x * 2
}

pub struct Alpha;

impl Alpha {
    pub fn make() -> Self {
        Alpha
    }
}
