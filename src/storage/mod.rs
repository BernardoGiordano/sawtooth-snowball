/*
 * Copyright 2018 Bitwise IO, Inc.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * -----------------------------------------------------------------------------
 */

//! Storage trait for syncing writes to an object to a backing store
//!
//! Hands out {read, write} RAII-guarded references to an object, and ensures
//! that when the reference drops, any changes to the object are persisted to
//! the selected storage.

pub mod disk;
pub mod memory;

use std::ops::{Deref, DerefMut};

use serde::de::DeserializeOwned;
use serde::Serialize;

pub use self::disk::DiskStorage;
pub use self::memory::MemStorage;

/// RAII structure used to allow read access to state object
///
/// This guard allows avoiding unnecessary syncing if you just need read
/// access to the state object.
pub trait StorageReadGuard<'a, T: Sized>: Deref<Target = T> {}

/// RAII structure used to allow write access to state object
///
/// This guard will ensure that any changes to an object are persisted to
/// a backing store when this is Dropped.
pub trait StorageWriteGuard<'a, T: Sized>: DerefMut<Target = T> {}

/// Storage wrapper that ensures that changes to an object are persisted to a backing store
///
/// Achieves this by handing out RAII-guarded references to the underlying data, that ensure
/// persistence when they get Dropped.
pub trait Storage {
    type S;

    fn read<'a>(&'a self) -> Box<dyn StorageReadGuard<'a, Self::S, Target = Self::S> + 'a>;
    fn write<'a>(&'a mut self) -> Box<dyn StorageWriteGuard<'a, Self::S, Target = Self::S> + 'a>;
}

/// Given a location string, returns the appropriate storage
///
/// Accepts `"memory"` or `"disk+/path/to/file"` as location values
pub fn get_storage<'a, T: Sized + Serialize + DeserializeOwned + 'a, F: Fn() -> T>(
    location: &str,
    default: F,
) -> Result<Box<dyn Storage<S = T> + 'a>, String> {
    if location == "memory" {
        Ok(Box::new(MemStorage::new(default)) as Box<dyn Storage<S = T>>)
    } else if location.starts_with("disk") {
        let split = location.splitn(2, '+').collect::<Vec<_>>();

        if split.len() != 2 {
            return Err(format!("Invalid location: {}", location));
        }

        Ok(Box::new(DiskStorage::from_path(split[1], default).unwrap()))
    } else {
        Err(format!("Unknown storage location type: {}", location))
    }
}