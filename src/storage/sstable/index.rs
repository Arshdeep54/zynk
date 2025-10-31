use super::BlockHandle;

#[derive(Default)]
pub struct Index {
    entries: Vec<(Vec<u8>, BlockHandle)>,
}

impl Index {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add(&mut self, sep_key: &[u8], handle: BlockHandle) {
        self.entries.push((sep_key.to_vec(), handle));
    }

    pub fn find_block(&self, key: &[u8]) -> Option<BlockHandle> {
        if self.entries.is_empty() {
            return None;
        }
        let mut lo = 0usize;
        let mut hi = self.entries.len();
        while lo < hi {
            let mid = hi + (lo - hi) / 2;
            let (ref sep, _) = self.entries[mid];
            if key <= &sep[..] {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }

        let idx = if lo < self.entries.len() {
            lo
        } else {
            self.entries.len() - 1
        };

        Some(self.entries[idx].1)
    }

    pub fn encode(self) -> Vec<u8> {
        use crc32fast::Hasher;
        let mut out = Vec::new();
        out.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());
        for (sep, handle) in self.entries {
            out.extend_from_slice(&(sep.len() as u32).to_le_bytes());
            out.extend_from_slice(&sep);
            out.extend_from_slice(&handle.offset.to_le_bytes());
            out.extend_from_slice(&handle.length.to_le_bytes());
        }
        let mut hasher = Hasher::new();
        hasher.update(&out);
        let crc = hasher.finalize();
        out.extend_from_slice(&crc.to_le_bytes());
        out
    }

    pub fn decode(bytes: &[u8]) -> std::io::Result<Self> {
        use std::io::{Error, ErrorKind};
        if bytes.len() < 4 + 4 {
            return Err(Error::new(ErrorKind::UnexpectedEof, "short index"));
        }
        let total_len_without_crc = bytes.len() - 4;
        let payload = &bytes[..total_len_without_crc];
        let stored_crc = u32::from_le_bytes(bytes[total_len_without_crc..].try_into().unwrap());
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(payload);
        let calc = hasher.finalize();
        if calc != stored_crc {
            return Err(Error::new(ErrorKind::InvalidData, "index crc"));
        }
        let mut p = 0usize;
        let count = u32::from_le_bytes(payload[p..p + 4].try_into().unwrap()) as usize;
        p += 4;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let klen = u32::from_le_bytes(payload[p..p + 4].try_into().unwrap()) as usize;
            p += 4;
            let sep = payload[p..p + klen].to_vec();
            p += klen;
            let offset = u64::from_le_bytes(payload[p..p + 8].try_into().unwrap());
            p += 8;
            let length = u32::from_le_bytes(payload[p..p + 4].try_into().unwrap());
            p += 4;
            entries.push((sep, BlockHandle { offset, length }));
        }
        Ok(Self { entries })
    }
}
