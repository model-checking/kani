// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// FIXME: This is a complete copy of `cargo/src/cargo/util/read2.rs`
// Consider unify the read2() in libstd, cargo and this to prevent further code duplication.

// Note: read2_abbreviated was replaced by read2 because the original function skipped some of the output,
// which caused issues with our tests (in particular tests with large outputs).

use std::io;
use std::process::{Child, Output};

pub fn read2(mut child: Child) -> io::Result<Output> {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    drop(child.stdin.take());
    self::imp::read2(
        child.stdout.take().unwrap(),
        child.stderr.take().unwrap(),
        &mut |is_stdout, data, _| {
            let out = if is_stdout { &mut stdout } else { &mut stderr };
            out.extend(data.iter());
            data.clear();
        },
    )?;
    let status = child.wait()?;

    Ok(Output { status, stdout, stderr })
}

#[cfg(not(any(unix, windows)))]
mod imp {
    use std::io::{self, Read};
    use std::process::{ChildStderr, ChildStdout};

    pub fn read2(
        out_pipe: ChildStdout,
        err_pipe: ChildStderr,
        data: &mut dyn FnMut(bool, &mut Vec<u8>, bool),
    ) -> io::Result<()> {
        let mut buffer = Vec::new();
        out_pipe.read_to_end(&mut buffer)?;
        data(true, &mut buffer, true);
        buffer.clear();
        err_pipe.read_to_end(&mut buffer)?;
        data(false, &mut buffer, true);
        Ok(())
    }
}

#[cfg(unix)]
mod imp {
    use std::io;
    use std::io::prelude::*;
    use std::mem;
    use std::os::unix::prelude::*;
    use std::process::{ChildStderr, ChildStdout};

    pub fn read2(
        mut out_pipe: ChildStdout,
        mut err_pipe: ChildStderr,
        data: &mut dyn FnMut(bool, &mut Vec<u8>, bool),
    ) -> io::Result<()> {
        unsafe {
            libc::fcntl(out_pipe.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK);
            libc::fcntl(err_pipe.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK);
        }

        let mut out_done = false;
        let mut err_done = false;
        let mut out = Vec::new();
        let mut err = Vec::new();

        let mut fds: [libc::pollfd; 2] = unsafe { mem::zeroed() };
        fds[0].fd = out_pipe.as_raw_fd();
        fds[0].events = libc::POLLIN;
        fds[1].fd = err_pipe.as_raw_fd();
        fds[1].events = libc::POLLIN;
        let mut nfds = 2;
        let mut errfd = 1;

        while nfds > 0 {
            // wait for either pipe to become readable using `select`
            let r = unsafe { libc::poll(fds.as_mut_ptr(), nfds, -1) };
            if r == -1 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(err);
            }

            // Read as much as we can from each pipe, ignoring EWOULDBLOCK or
            // EAGAIN. If we hit EOF, then this will happen because the underlying
            // reader will return Ok(0), in which case we'll see `Ok` ourselves. In
            // this case we flip the other fd back into blocking mode and read
            // whatever's leftover on that file descriptor.
            let handle = |res: io::Result<_>| match res {
                Ok(_) => Ok(true),
                Err(e) => {
                    if e.kind() == io::ErrorKind::WouldBlock {
                        Ok(false)
                    } else {
                        Err(e)
                    }
                }
            };
            if !err_done && fds[errfd].revents != 0 && handle(err_pipe.read_to_end(&mut err))? {
                err_done = true;
                nfds -= 1;
            }
            data(false, &mut err, err_done);
            if !out_done && fds[0].revents != 0 && handle(out_pipe.read_to_end(&mut out))? {
                out_done = true;
                fds[0].fd = err_pipe.as_raw_fd();
                errfd = 0;
                nfds -= 1;
            }
            data(true, &mut out, out_done);
        }
        Ok(())
    }
}
