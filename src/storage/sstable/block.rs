use crc32fast::Hasher;

pub struct DataBlock {
    target_bytes: usize,
    payload: Vec<u8>,
    entries: usize,
}

impl DataBlock {
    pub fn new(target_bytes: usize) -> Self {
        Self {
            target_bytes,
            payload: Vec::with_capacity(target_bytes),
            entries: 0,
        }
    }

    pub fn add_put(&mut self, key: &[u8], value: &[u8]) {
        self.payload.push(0);
        self.payload
            .extend_from_slice(&(key.len() as u32).to_le_bytes());
        self.payload
            .extend_from_slice(&(value.len() as u32).to_le_bytes());
        self.payload.extend_from_slice(key);
        self.payload.extend_from_slice(value);
        self.entries += 1;
    }

    pub fn add_delete(&mut self, key: &[u8]) {
        self.payload.push(1);
        self.payload
            .extend_from_slice(&(key.len() as u32).to_le_bytes());
        self.payload.extend_from_slice(&0u32.to_le_bytes());
        self.payload.extend_from_slice(key);
        self.entries += 1;
    }

    pub fn is_full(&self) -> bool {
        self.payload.len() >= self.target_bytes && self.entries > 0
    }

    pub fn encode(self) -> Vec<u8> {
        let mut hasher = Hasher::new();
        hasher.update(&self.payload);
        let crc = hasher.finalize();
        let mut out = self.payload;
        out.extend_from_slice(&crc.to_le_bytes());
        out
    }

    pub fn len(&self) -> usize {
        self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries == 0
    }
}
