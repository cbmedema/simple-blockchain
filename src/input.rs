#[derive(Clone, Copy, Hash)]
pub struct Input {
    pub txid: [u8;32],
    pub signature: [u8;64],
}