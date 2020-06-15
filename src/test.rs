use crate::{generation::*, loader::*, map::*, *};
use manager::ManagedGen;
use std::{any::TypeId, thread};

#[derive(Clone, Debug)]
struct TestLoader(pub GenericReceiver<String>);

impl TestLoader {
    fn receive(&self) {
        for (_, into) in self.0.iter() {
            let item = match into.meta_data {
                x if x == TypeId::of::<f32>() => GenericItem::new(101.1_f32),
                x if x == TypeId::of::<u32>() => GenericItem::new(55_u32),
                x if x == TypeId::of::<i32>() => GenericItem::new(-21_i32),
                _ => GenericItem::new(false),
            };

            into.send(item).expect("Failed to send test data");
        }
    }
}

#[test]
fn mapped_generation_test() {
    let mut storage = MappedGeneration::new();
    storage.insert("Beta".to_string(), "not a cool dude".to_string());
    storage.insert("Gamma".to_string(), "a loser".to_string());
    storage.insert("Alpha".to_string(), "what a cool dude".to_string());

    let mut key_idx: KeyIdx<String, StorageId> = KeyIdx::new("Alpha".to_string());
    storage.fill_key_idx_get(&mut key_idx);
    let removed = storage.remove(&key_idx).unwrap();

    storage.insert("Alpha".to_string(), removed);
    let key = key_idx.into_key();
    let mut key_idx: KeyIdx<String, StorageId> = KeyIdx::new(key);
    storage.fill_key_idx_get(&mut key_idx);
    let item = storage.get(&key_idx);

    assert!(
        KeyIdx {
            key: "Alpha".to_string(),
            index: Some(StorageId {
                index: 2,
                generation: 1
            })
        } == key_idx
    );

    assert!(item.map(|item| &item[..]) == Some("what a cool dude"));
}

#[test]
fn loader_test() {
    let (sender, receiver) = cbc::unbounded();
    let test_loader = TestLoader(receiver);

    thread::spawn(move || {
        test_loader.receive();
    });

    let mut floats: GenLoader<String, f32> = GenLoader::new_with_loader(sender.clone());
    let mut uints: GenLoader<String, u32> = GenLoader::new_with_loader(sender.clone());
    let mut ints: GenLoader<String, i32> = GenLoader::new_with_loader(sender.clone());

    let mut fkey = KeyIdx::new("test".to_string());
    let mut ukey = KeyIdx::new("test".to_string());
    let mut ikey = KeyIdx::new("test".to_string());
    floats.load(&mut fkey);
    uints.load(&mut ukey);
    ints.load(&mut ikey);

    floats.update_block_loading().unwrap();
    uints.update_block_loading().unwrap();
    ints.update_block_loading().unwrap();

    assert!(floats.get(&fkey) == Some(&101.1_f32));
    assert!(uints.get(&ukey) == Some(&55_u32));
    assert!(ints.get(&ikey) == Some(&-21_i32));
}

#[test]
fn retain_test() {
    let mut storage = MappedGeneration::new();
    let (a, _) = storage.insert("A", 100);
    let (b, _) = storage.insert("B", 100);
    let (c, _) = storage.insert("C", 200);
    let (d, _) = storage.insert("D", 300);

    storage.retain(|_, value| *value != 100);

    println!("{:?}", storage);
    assert!(storage.get_by_index(&a) == None);
    assert!(storage.get_by_index(&b) == None);
    assert!(storage.get_by_index(&c) == Some(&200));
    assert!(storage.get_by_index(&d) == Some(&300));
}

#[test]
fn manager_test() {
    let (sender, receiver) = cbc::unbounded();
    let test_loader = TestLoader(receiver);

    thread::spawn(move || {
        test_loader.receive();
    });

    let mut manager = ManagedGen::<String, f32, u32>::new(GenLoader::new_with_loader(sender), 3);

    let mut a = KeyIdx::new("a".to_string());
    manager.load(&mut a);
    manager.update_loaded_blocking().unwrap();
    manager.increment(&1);


    let mut b = KeyIdx::new("b".to_string());
    manager.load(&mut b);
    manager.update_loaded_blocking().unwrap();
    manager.increment(&1);

    let mut c = KeyIdx::new("c".to_string());
    manager.load(&mut c);
    manager.update_loaded_blocking().unwrap();
    manager.increment(&1);

    assert!(manager.get(&a) == None);
    assert!(manager.get(&b) == Some(&101.1_f32));
    assert!(manager.get(&c) == Some(&101.1_f32));

    manager.increment(&1);

    assert!(manager.get(&a) == None);
    assert!(manager.get(&b) == None);
    assert!(manager.get(&c) == Some(&101.1_f32));

    manager.increment(&1);

    assert!(manager.get(&a) == None);
    assert!(manager.get(&b) == None);
    assert!(manager.get(&c) == None);
}
