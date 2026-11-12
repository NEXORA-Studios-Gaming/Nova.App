use crate::{Error, Result};
use serde::{Serialize, de::DeserializeOwned};
use toml_edit::{DocumentMut, Item, Key, Table};

pub(crate) fn keys(path: &str) -> Result<Vec<String>> {
    let keys = Key::parse(path).map_err(|_| Error::InvalidKey(path.into()))?;
    if keys.is_empty() {
        return Err(Error::InvalidKey(path.into()));
    }
    Ok(keys.into_iter().map(|k| k.get().to_owned()).collect())
}

pub(crate) fn get<'a>(mut item: &'a Item, keys: &[String], path: &str) -> Result<&'a Item> {
    for key in keys {
        let table = item
            .as_table_like()
            .ok_or_else(|| Error::NotATable(path.into()))?;
        item = table
            .get(key)
            .ok_or_else(|| Error::MissingKey(path.into()))?;
    }
    Ok(item)
}

pub(crate) fn value<V: Serialize + ?Sized>(value: &V) -> Result<Item> {
    #[derive(Serialize)]
    struct Wrapper<'a, T: ?Sized> {
        value: &'a T,
    }
    toml_edit::ser::to_document(&Wrapper { value })?
        .as_table_mut()
        .remove("value")
        .ok_or(Error::UnsupportedValue)
}

pub(crate) fn decode_value<V: DeserializeOwned>(
    item: &Item,
) -> std::result::Result<V, toml_edit::de::Error> {
    #[derive(serde::Deserialize)]
    struct Wrapper<T> {
        value: T,
    }
    let mut doc = DocumentMut::new();
    doc.insert("value", item.clone());
    toml_edit::de::from_document::<Wrapper<V>>(doc).map(|wrapper| wrapper.value)
}

fn replace(target: &mut Item, mut replacement: Item) {
    // A value inside an inline table must remain a Value, even when replaced by a struct.
    if target.as_value().is_some() {
        replacement = match replacement.into_value() {
            Ok(value) => Item::Value(value),
            Err(item) => item,
        };
    }
    // Key formatting belongs to its containing table and survives get_mut.
    if let (Some(before), Some(after)) = (target.as_value(), replacement.as_value_mut()) {
        *after.decor_mut() = before.decor().clone();
    }
    if let (Some(before), Some(after)) = (target.as_table(), replacement.as_table_mut()) {
        *after.decor_mut() = before.decor().clone();
    }
    *target = replacement;
}

pub(crate) fn set(item: &mut Item, keys: &[String], replacement: Item, path: &str) -> Result<()> {
    let table = item
        .as_table_like_mut()
        .ok_or_else(|| Error::NotATable(path.into()))?;
    let key = &keys[0];
    if keys.len() == 1 {
        if let Some(target) = table.get_mut(key) {
            replace(target, replacement);
        } else {
            table.insert(key, replacement);
        }
        return Ok(());
    }
    if !table.contains_key(key) {
        let mut child = Table::new();
        child.set_implicit(true);
        table.insert(key, Item::Table(child));
    }
    set(
        table.get_mut(key).expect("key inserted above"),
        &keys[1..],
        replacement,
        path,
    )
}

pub(crate) fn remove(item: &mut Item, keys: &[String], path: &str) -> Result<bool> {
    let table = item
        .as_table_like_mut()
        .ok_or_else(|| Error::NotATable(path.into()))?;
    if keys.len() == 1 {
        return Ok(table.remove(&keys[0]).is_some());
    }
    match table.get_mut(&keys[0]) {
        Some(child) => remove(child, &keys[1..], path),
        None => Ok(false),
    }
}

/// Apply only typed changes, retaining unmodeled keys and untouched source formatting.
pub(crate) fn merge_changes(target: &mut Item, old: &Item, new: &Item) {
    if old.to_string() == new.to_string() {
        return;
    }
    if let (Some(old_table), Some(new_table), Some(target_table)) = (
        old.as_table_like(),
        new.as_table_like(),
        target.as_table_like_mut(),
    ) {
        for (key, _) in old_table.iter() {
            if !new_table.contains_key(key) {
                target_table.remove(key);
            }
        }
        for (key, new_value) in new_table.iter() {
            match old_table.get(key) {
                Some(old_value) if old_value.to_string() == new_value.to_string() => {}
                Some(old_value) => {
                    if let Some(target_value) = target_table.get_mut(key) {
                        merge_changes(target_value, old_value, new_value);
                    } else {
                        target_table.insert(key, new_value.clone());
                    }
                }
                None => {
                    target_table.insert(key, new_value.clone());
                }
            }
        }
    } else {
        replace(target, new.clone());
    }
}
