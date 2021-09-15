//! Tidy checks source code in this repository.
//!
//! This program runs all of the various tidy checks for style, cleanliness,
//! etc. This is run by default on `./x.py test` and as part of the auto
//! builders. The tidy checks can be executed with `./x.py test tidy`.

use tidy::*;

use crossbeam_utils::thread::{scope, ScopedJoinHandle};
use std::collections::VecDeque;
use std::env;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::process;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};

fn main() {
    let root_path: PathBuf = env::args_os().nth(1).expect("need path to root of repo").into();
    let cargo: PathBuf = env::args_os().nth(2).expect("need path to cargo").into();
    let output_directory: PathBuf =
        env::args_os().nth(3).expect("need path to output directory").into();
    let concurrency: NonZeroUsize =
        FromStr::from_str(&env::args().nth(4).expect("need concurrency"))
            .expect("concurrency must be a number");

    let src_path = root_path.join("src");
    let library_path = root_path.join("library");
    let compiler_path = root_path.join("compiler");

    let args: Vec<String> = env::args().skip(1).collect();

    let verbose = args.iter().any(|s| *s == "--verbose");

    let bad = std::sync::Arc::new(AtomicBool::new(false));

    scope(|s| {
        let mut handles: VecDeque<ScopedJoinHandle<'_, ()>> =
            VecDeque::with_capacity(concurrency.get());

        macro_rules! check {
            ($p:ident $(, $args:expr)* ) => {
                while handles.len() >= concurrency.get() {
                    handles.pop_front().unwrap().join().unwrap();
                }

                let handle = s.spawn(|_| {
                    let mut flag = false;
                    $p::check($($args),* , &mut flag);
                    if (flag) {
                        bad.store(true, Ordering::Relaxed);
                    }
                });
                handles.push_back(handle);
            }
        }

        check!(target_specific_tests, &src_path);

        // Checks that are done on the cargo workspace.
        check!(deps, &root_path, &cargo);
        check!(extdeps, &root_path);

        // Checks over tests.
        check!(debug_artifacts, &src_path);
        check!(ui_tests, &src_path);

        // Checks that only make sense for the compiler.
        check!(errors, &compiler_path);
        check!(error_codes_check, &[&src_path, &compiler_path]);

        // Checks that only make sense for the std libs.
        check!(pal, &library_path);
        check!(primitive_docs, &library_path);

        // Checks that need to be done for both the compiler and std libraries.
        check!(unit_tests, &src_path);
        check!(unit_tests, &compiler_path);
        check!(unit_tests, &library_path);

        if bins::check_filesystem_support(
            &[&src_path, &compiler_path, &library_path],
            &output_directory,
        ) {
            check!(bins, &src_path);
            check!(bins, &compiler_path);
            check!(bins, &library_path);
        }

        check!(style, &src_path);
        check!(style, &compiler_path);
        check!(style, &library_path);

        check!(edition, &src_path);
        check!(edition, &compiler_path);
        check!(edition, &library_path);

        let collected = {
            while handles.len() >= concurrency.get() {
                handles.pop_front().unwrap().join().unwrap();
            }
            let mut flag = false;
            let r = features::check(&src_path, &compiler_path, &library_path, &mut flag, verbose);
            if flag {
                bad.store(true, Ordering::Relaxed);
            }
            r
        };
        check!(unstable_book, &src_path, collected);
    })
    .unwrap();

    if bad.load(Ordering::Relaxed) {
        eprintln!("some tidy checks failed");
        process::exit(1);
    }
}
