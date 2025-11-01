use crate::storage::memtable::{Entry, MemTable, MemTableSet, flush_memtable_to_sstable};
use crate::storage::sstable::{TableId, reader::SsTableReader};
use std::fs;
use std::io::Result;
use std::path::{Path, PathBuf};

pub struct LsmEngine {
    data_dir: PathBuf,
    memtables: MemTableSet,
    sstables: Vec<(TableId, PathBuf, SsTableReader)>,
    block_bytes: usize,
    next_table_id: TableId,
}

impl LsmEngine {
    pub fn new<P: AsRef<Path>>(
        data_dir: P,
        memtable_max_bytes: usize,
        block_bytes: usize,
    ) -> std::io::Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        let sst_dir = data_dir.join("sst");
        fs::create_dir_all(&sst_dir)?;
        let memtables = MemTableSet::with_capacity(memtable_max_bytes);
        Ok(Self {
            data_dir,
            memtables,
            sstables: Vec::new(),
            block_bytes,
            next_table_id: 1,
        })
    }

    pub fn put(&mut self, key: &[u8], value: &[u8]) -> std::io::Result<()> {
        if let Some(frozen) = self.memtables.put(key, value) {
            self.flush_immutable(frozen)?;
        }
        Ok(())
    }

    pub fn delete(&mut self, key: &[u8]) -> std::io::Result<()> {
        if let Some(frozen) = self.memtables.delete(key) {
            self.flush_immutable(frozen)?;
        }
        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> std::io::Result<Option<Vec<u8>>> {
        if let Some(entry) = self.memtables.get(key) {
            return Ok(match entry {
                Entry::Put(v) => Some(v.clone()),
                Entry::Delete => None,
            });
        }
        for (_, _path, reader) in self.sstables.iter().rev() {
            if let Some(v) = reader.get(key)? {
                return Ok(Some(v));
            }
        }
        Ok(None)
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        if let Some(frozen) = self.memtables.rotate() {
            self.flush_immutable(frozen)?;
        }
        Ok(())
    }

    fn flush_immutable(&mut self, frozen: MemTable) -> Result<()> {
        let id = self.alloc_table_id();
        let tmp = self.sst_tmp_path(id);
        let final_path = self.sst_final_path(id);

        let _ = fs::create_dir_all(final_path.parent().unwrap());
        let _res = flush_memtable_to_sstable(frozen, &tmp, self.block_bytes)?;

        fs::rename(&tmp, &final_path)?;

        let reader = SsTableReader::open(&final_path)?;
        self.sstables.push((id, final_path, reader));
        Ok(())
    }

    fn sst_tmp_path(&self, id: TableId) -> PathBuf {
        self.data_dir.join("sst").join(format!("{id:06}.sst.tmp"))
    }

    fn sst_final_path(&self, id: TableId) -> PathBuf {
        self.data_dir.join("sst").join(format!("{id:06}.sst"))
    }

    fn alloc_table_id(&mut self) -> TableId {
        let id = self.next_table_id;
        self.next_table_id += 1;
        id
    }
}
