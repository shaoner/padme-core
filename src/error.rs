#[cfg_attr(debug_assertions, derive(Debug))]
pub enum Error {
    InvalidRomSize(usize),
}
