use crate::{generation::*, map::*, *};

#[test]
fn mapped_generation_test() {
    let mut storage = MappedGeneration::new();
    storage.insert("Beta".to_string(), "not a cool dude".to_string());
    storage.insert("Gamma".to_string(), "a loser".to_string());
    storage.insert("Alpha".to_string(), "what a cool dude".to_string());

    let mut key_idx: KeyIdx<String, StorageId> = KeyIdx::Key("Alpha".to_string());
    storage.fill_key_idx_get(&mut key_idx);
    let removed = storage.remove(&key_idx).unwrap();

    storage.insert("Alpha".to_string(), removed);
    let key = key_idx.into_key().unwrap();
    let mut key_idx: KeyIdx<String, StorageId> = KeyIdx::new(Some(key), None).unwrap();
    storage.fill_key_idx_get(&mut key_idx);
    let item = storage.get(&key_idx);

    assert!(
        KeyIdx::Both {
            key: "Alpha".to_string(),
            index: StorageId {
                index: 2,
                generation: 1
            }
        } == key_idx
    );

    assert!(
        item.map(|item| &item[..]) == Some("what a cool dude")
    );
}
