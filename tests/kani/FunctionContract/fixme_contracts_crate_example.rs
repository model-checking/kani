/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */
extern crate kani;
use kani::*;
use std::collections::HashSet;

pub fn implies(premise: bool, conclusion: bool) -> bool {
    !premise || conclusion
}

pub struct Library {
    available: HashSet<String>,
    lent: HashSet<String>,
}

impl Library {
    fn book_exists(&self, book_id: &str) -> bool {
        self.available.contains(book_id) || self.lent.contains(book_id)
    }

    #[requires(!self.book_exists(book_id))]
    #[ensures(self.available.contains(book_id))]
    //#[ensures(self.available.len() == old(self.available.len()) + 1)]
    //#[ensures(self.lent.len() == old(self.lent.len()))]
    pub fn add_book(&mut self, book_id: &str) {
        self.available.insert(book_id.to_string());
    }

    #[requires(self.book_exists(book_id))]
    //#[ensures(implies(result, self.available.len() == old(self.available.len()) - 1))]
    //#[ensures(implies(result, self.lent.len() == old(self.lent.len()) + 1))]
    #[ensures(implies(result, self.lent.contains(book_id)))]
    #[ensures(implies(!result, self.lent.contains(book_id)))]
    pub fn lend(&mut self, book_id: &str) -> bool {
        if self.available.contains(book_id) {
            self.available.remove(book_id);
            self.lent.insert(book_id.to_string());
            true
        } else {
            false
        }
    }

    #[requires(self.lent.contains(book_id))]
    //#[ensures(self.lent.len() == old(self.lent.len()) - 1)]
    //#[ensures(self.available.len() == old(self.available.len()) + 1)]
    #[ensures(!self.lent.contains(book_id))]
    #[ensures(self.available.contains(book_id))]
    pub fn return_book(&mut self, book_id: &str) {
        self.lent.remove(book_id);
        self.available.insert(book_id.to_string());
    }
}

#[kani::proof]
fn main() {
    let mut lib = Library {
        available: HashSet::new(),
        lent: HashSet::new(),
    };

    lib.add_book("Das Kapital");
    println!("Adding a book.");

    let lent_successful = lib.lend("Das Kapital");
    assert!(lent_successful);

    if lent_successful {
        println!("Lent out the book.");
        println!("Reading for a bit...");

        println!("Giving back the book.");

        lib.return_book("Das Kapital");
    }
}

