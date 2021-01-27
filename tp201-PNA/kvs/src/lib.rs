// just leave it empty for now

#[derive(Clone, Debug, Default)]
pub struct KvStore;

impl KvStore {
    pub fn new() -> KvStore {
        KvStore::default()
    }

    pub fn get(&self, key: String) -> Option<String> {
        unimplemented!()
    }

    pub fn set(&mut self, key: String, val: String) {
        unimplemented!()
    }

    pub fn remove(&mut self, key: String) {
        unimplemented!()
    }
}
