/// An iterator yielding key-value pairs from an SSTable in sorted order.
pub struct SsTableIter;

impl SsTableIter {
    /// Creates a new iterator for the given reader starting at an optional key.
    pub fn new_seek(_start: Option<&[u8]>) -> Self {
        unimplemented!()
    }
}

impl Iterator for SsTableIter {
    type Item = (Vec<u8>, Vec<u8>);

    /// Advances the iterator and returns the next item if any.
    fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}
