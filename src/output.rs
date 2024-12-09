#[derive(Clone, Hash)]
pub struct Output {
    pub amount: u64,
    pub address: [u8;32],
}