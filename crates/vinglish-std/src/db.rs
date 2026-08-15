use vinglish_bindgen::vinglish_bindgen;
use sled::Db;
use std::sync::OnceLock;

fn get_db(_collection: &str) -> Db {
    static DB_CACHE: OnceLock<sled::Db> = OnceLock::new();
    // In a real scenario, we'd want collections. Sled uses Trees for this.
    // For simplicity, we just open a `.vinglish_db` folder and use `open_tree`.
    let db = DB_CACHE.get_or_init(|| {
        sled::open(".vinglish_db").expect("Failed to open database")
    });
    // We just return the main db for now, trees can be used if we want
    db.clone()
}

#[vinglish_bindgen]
pub fn db_put(collection: &str, key: &str, value: &str) -> bool {
    let db = get_db(collection);
    let tree = db.open_tree(collection).unwrap();
    tree.insert(key, value).is_ok()
}

#[vinglish_bindgen]
pub fn db_get(collection: &str, key: &str) -> String {
    let db = get_db(collection);
    if let Ok(tree) = db.open_tree(collection) {
        if let Ok(Some(value)) = tree.get(key) {
            return String::from_utf8_lossy(&value).to_string();
        }
    }
    String::new()
}

#[vinglish_bindgen]
pub fn db_delete(collection: &str, key: &str) -> bool {
    let db = get_db(collection);
    if let Ok(tree) = db.open_tree(collection) {
        return tree.remove(key).is_ok();
    }
    false
}

#[vinglish_bindgen]
pub fn db_contains(collection: &str, key: &str) -> bool {
    let db = get_db(collection);
    if let Ok(tree) = db.open_tree(collection) {
        return tree.contains_key(key).unwrap_or(false);
    }
    false
}

#[vinglish_bindgen]
pub fn db_batch_put(collection: &str, keys_and_values_json: &str) -> bool {
    let db = get_db(collection);
    if let Ok(tree) = db.open_tree(collection) {
        if let Ok(json) = serde_json::from_str::<std::collections::HashMap<String, String>>(keys_and_values_json) {
            let mut batch = sled::Batch::default();
            for (k, v) in json {
                batch.insert(k.as_bytes(), v.as_bytes());
            }
            return tree.apply_batch(batch).is_ok();
        }
    }
    false
}
