pub trait CRDT: Sized {
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Self;
    fn merge(&mut self, other: &Self);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GSet {
    elems: Vec<Vec<u8>>, 
}

impl GSet {
    pub fn new() -> Self { Self { elems: Vec::new() } }

    pub fn contains(&self, k: &[u8]) -> bool {
        self.elems.binary_search_by(|e| e.as_slice().cmp(k)).is_ok()
    }

    pub fn insert(&mut self, k: Vec<u8>) {
        match self.elems.binary_search(&k) {
            Ok(_) => {}
            Err(i) => self.elems.insert(i, k),
        }
    }

    pub fn len(&self) -> usize { self.elems.len() }

    pub fn iter(&self) -> impl Iterator<Item=&Vec<u8>> { self.elems.iter() }

    pub fn elements(&self) -> Vec<Vec<u8>> {
        self.elems.clone()
    }
}

impl CRDT for GSet {
    fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(&(self.elems.len() as u32).to_be_bytes());
        for e in &self.elems {
            out.extend(&(e.len() as u32).to_be_bytes());
            out.extend(e);
        }
        out
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        use std::convert::TryInto;
        let mut i = 0;
        if bytes.len() < 4 { return GSet::new(); }
        let cnt = u32::from_be_bytes(bytes[i..i+4].try_into().unwrap()) as usize; i += 4;
        let mut elems = Vec::with_capacity(cnt);
        for _ in 0..cnt {
            if i + 4 > bytes.len() { break; }
            let l = u32::from_be_bytes(bytes[i..i+4].try_into().unwrap()) as usize; i += 4;
            if i + l > bytes.len() { break; }
            elems.push(bytes[i..i+l].to_vec());
            i += l;
        }
        elems.sort();
        elems.dedup();
        GSet { elems }
    }

    fn merge(&mut self, other: &Self) {
        let mut out = Vec::with_capacity(self.elems.len() + other.elems.len());
        let mut a = &self.elems[..];
        let mut b = &other.elems[..];
        let mut ia = 0usize;
        let mut ib = 0usize;
        while ia < a.len() && ib < b.len() {
            let av = &a[ia];
            let bv = &b[ib];
            match av.cmp(bv) {
                std::cmp::Ordering::Less => { out.push(av.clone()); ia += 1; }
                std::cmp::Ordering::Greater => { out.push(bv.clone()); ib += 1; }
                std::cmp::Ordering::Equal => { out.push(av.clone()); ia += 1; ib += 1; }
            }
        }
        while ia < a.len() { out.push(a[ia].clone()); ia += 1; }
        while ib < b.len() { out.push(b[ib].clone()); ib += 1; }
        self.elems = out;
    }
}