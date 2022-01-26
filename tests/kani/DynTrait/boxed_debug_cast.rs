// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we can cast &&Box<dyn Error + Send + Sync> to &dyn Debug
// without panicing

use std::error::Error;
use std::fmt::{Debug, Display, Formatter, Result};

#[derive(Debug)]
struct Concrete;
impl Error for Concrete {}

impl Display for Concrete {
    fn fmt(&self, f: &mut Formatter) -> Result {
        Ok(())
    }
}

fn f<'a>(x: &'a &Box<dyn Error + Send + Sync>) -> Box<&'a dyn Debug> {
    let d = x as &dyn Debug;
    Box::new(d)
}

fn main() {
    let c = Concrete {};
    let x = Box::new(c) as Box<dyn Error + Send + Sync>;
    let d = f(&&x);
}
