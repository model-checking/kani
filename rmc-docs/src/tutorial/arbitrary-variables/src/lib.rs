// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::collections::HashMap;
use std::num::NonZeroU32;

struct Inventory {
    inner: HashMap<String, NonZeroU32>,
}

impl Inventory {
    pub fn update(&mut self, name: String, new_quantity: NonZeroU32) {
        self.inner.insert(name, new_quantity);
    }

    pub fn get(&self, name: &String) -> Option<&NonZeroU32> {
        self.inner.get(name)
    }
}

#[cfg(rmc)]
mod verification {
    use super::*;

    #[rmc::proof]
    pub fn safe_update() {
        let mut inventory = Inventory { inner: HashMap::default() };
        let name = String::from("product1");
        let quantity = rmc::any();

        inventory.update(name.clone(), quantity);
        assert!(inventory.get(&name).is_some());
        assert!(*inventory.get(&name).unwrap() == quantity);
    }
}
