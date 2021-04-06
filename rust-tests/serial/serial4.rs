// Copyright 2018 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
//
// Portions Copyright 2017 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the THIRD-PARTY file.

use std::collections::VecDeque;

const LOOP_SIZE: usize = 0x40;

const DATA: u8 = 0;
const IER: u8 = 1;
const IIR: u8 = 2;
const LCR: u8 = 3;
const MCR: u8 = 4;
const LSR: u8 = 5;
const MSR: u8 = 6;
const SCR: u8 = 7;

const DLAB_LOW: u8 = 0;
const DLAB_HIGH: u8 = 1;

const IER_RECV_BIT: u8 = 0x1;
const IER_THR_BIT: u8 = 0x2;
const IER_FIFO_BITS: u8 = 0x0f;

const IIR_FIFO_BITS: u8 = 0xc0;
const IIR_NONE_BIT: u8 = 0x1;
const IIR_THR_BIT: u8 = 0x2;
const IIR_RECV_BIT: u8 = 0x4;

const LCR_DLAB_BIT: u8 = 0x80;

const LSR_DATA_BIT: u8 = 0x1;
const LSR_EMPTY_BIT: u8 = 0x20;
const LSR_IDLE_BIT: u8 = 0x40;

const MCR_LOOP_BIT: u8 = 0x10;

const DEFAULT_INTERRUPT_IDENTIFICATION: u8 = IIR_NONE_BIT; // no pending interrupt
const DEFAULT_LINE_STATUS: u8 = LSR_EMPTY_BIT | LSR_IDLE_BIT; // THR empty and line is idle
const DEFAULT_LINE_CONTROL: u8 = 0x3; // 8-bits per character
const DEFAULT_MODEM_CONTROL: u8 = 0x8; // Auxiliary output 2
const DEFAULT_MODEM_STATUS: u8 = 0x20 | 0x10 | 0x80; // data ready, clear to send, carrier detect
const DEFAULT_BAUD_DIVISOR: u16 = 12; // 9600 bps

// Cannot use multiple types as bounds for a trait object, so we define our own trait
// which is a composition of the desired bounds. In this case, io::Read and AsRawFd.
// Run `rustc --explain E0225` for more details.
// /// Trait that composes the `std::io::Read` and `std::os::unix::io::AsRawFd` traits.
// pub trait ReadableFd: io::Read + AsRawFd {}

// overwrite the following types

#[derive(Clone)]
pub struct OutWrapper {
    output: Vec<u8>,
}

impl OutWrapper {
    fn new() -> OutWrapper {
        OutWrapper { output: Vec::new() }
    }

    //write_all
    //flush

    fn write_all(&mut self, buf: &[u8]) -> Result<(), usize> {
        for b in buf {
            self.output.push(*b);
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), usize> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct InputWrapper {}

impl InputWrapper {
    //read

    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, usize> {
        unimplemented!()
    }

    fn as_raw_fd(&self) -> u64 {
        unimplemented!()
    }
}

pub struct EventFd {}

impl EventFd {
    pub fn write(&self, _v: u64) -> Result<(), usize> {
        //change from io::Error to usize
        // unimplemented!()
        Ok(())
    }
}

/// Emulates serial COM ports commonly seen on x86 I/O ports 0x3f8/0x2f8/0x3e8/0x2e8.
///
/// This can optionally write the guest's output to a Write trait object. To send input to the
/// guest, use `raw_input`.
pub struct Serial {
    interrupt_enable: u8,
    interrupt_identification: u8,
    interrupt_evt: EventFd,
    line_control: u8,
    line_status: u8,
    modem_control: u8,
    modem_status: u8,
    scratch: u8,
    baud_divisor: u16,
    in_buffer: VecDeque<u8>,
    out: Option<OutWrapper>,
    input: Option<InputWrapper>,
}

impl Serial {
    fn new(interrupt_evt: EventFd, out: Option<OutWrapper>, input: Option<InputWrapper>) -> Serial {
        let interrupt_enable = match out {
            Some(_) => IER_RECV_BIT,
            None => 0,
        };
        Serial {
            interrupt_enable,
            interrupt_identification: DEFAULT_INTERRUPT_IDENTIFICATION,
            interrupt_evt,
            line_control: DEFAULT_LINE_CONTROL,
            line_status: DEFAULT_LINE_STATUS,
            modem_control: DEFAULT_MODEM_CONTROL,
            modem_status: DEFAULT_MODEM_STATUS,
            scratch: 0,
            baud_divisor: DEFAULT_BAUD_DIVISOR,
            in_buffer: VecDeque::new(),
            out,
            input,
        }
    }

    /// Constructs a Serial port ready for input and output.
    pub fn new_in_out(interrupt_evt: EventFd, input: InputWrapper, out: OutWrapper) -> Serial {
        Self::new(interrupt_evt, Some(out), Some(input))
    }

    /// Constructs a Serial port ready for output but with no input.
    pub fn new_out(interrupt_evt: EventFd, out: OutWrapper) -> Serial {
        Self::new(interrupt_evt, Some(out), None)
    }

    /// Constructs a Serial port with no connected input or output.
    pub fn new_sink(interrupt_evt: EventFd) -> Serial {
        Self::new(interrupt_evt, None, None)
    }

    /// Provides a reference to the interrupt event fd.
    pub fn interrupt_evt(&self) -> &EventFd {
        &self.interrupt_evt
    }

    fn is_dlab_set(&self) -> bool {
        (self.line_control & LCR_DLAB_BIT) != 0
    }

    fn is_recv_intr_enabled(&self) -> bool {
        (self.interrupt_enable & IER_RECV_BIT) != 0
    }

    fn is_thr_intr_enabled(&self) -> bool {
        (self.interrupt_enable & IER_THR_BIT) != 0
    }

    fn is_loop(&self) -> bool {
        (self.modem_control & MCR_LOOP_BIT) != 0
    }

    fn add_intr_bit(&mut self, bit: u8) {
        self.interrupt_identification &= !IIR_NONE_BIT;
        self.interrupt_identification |= bit;
    }

    fn del_intr_bit(&mut self, bit: u8) {
        self.interrupt_identification &= !bit;
        if self.interrupt_identification == 0x0 {
            self.interrupt_identification = IIR_NONE_BIT;
        }
    }

    fn thr_empty_interrupt(&mut self) -> Result<(), usize> {
        if self.is_thr_intr_enabled() {
            self.add_intr_bit(IIR_THR_BIT);
            self.interrupt_evt.write(1)?;
        }
        Ok(())
    }

    fn recv_data_interrupt(&mut self) -> Result<(), usize> {
        if self.is_recv_intr_enabled() {
            self.add_intr_bit(IIR_RECV_BIT);
            self.interrupt_evt.write(1)?
        }
        self.line_status |= LSR_DATA_BIT;
        Ok(())
    }

    fn iir_reset(&mut self) {
        self.interrupt_identification = DEFAULT_INTERRUPT_IDENTIFICATION;
    }

    // Handles a write request from the driver.
    fn handle_write(&mut self, offset: u8, value: u8) -> Result<(), usize> {
        match offset as u8 {
            DLAB_LOW if self.is_dlab_set() => {
                self.baud_divisor = (self.baud_divisor & 0xff00) | u16::from(value)
            }
            DLAB_HIGH if self.is_dlab_set() => {
                self.baud_divisor = (self.baud_divisor & 0x00ff) | (u16::from(value) << 8)
            }
            DATA => {
                if self.is_loop() {
                    if self.in_buffer.len() < LOOP_SIZE {
                        self.in_buffer.push_back(value);
                        self.recv_data_interrupt()?;
                    }
                } else {
                    if let Some(out) = self.out.as_mut() {
                        out.write_all(&[value])?;
                        // METRICS.uart.write_count.inc();
                        out.flush()?;
                        // METRICS.uart.flush_count.inc();
                    }
                    self.thr_empty_interrupt()?;
                }
            }
            IER => self.interrupt_enable = value & IER_FIFO_BITS,
            LCR => self.line_control = value,
            MCR => self.modem_control = value,
            SCR => self.scratch = value,
            _ => {}
        }
        Ok(())
    }

    // Handles a read request from the driver.
    fn handle_read(&mut self, offset: u8) -> u8 {
        match offset as u8 {
            DLAB_LOW if self.is_dlab_set() => self.baud_divisor as u8,
            DLAB_HIGH if self.is_dlab_set() => (self.baud_divisor >> 8) as u8,
            DATA => {
                self.del_intr_bit(IIR_RECV_BIT);
                if self.in_buffer.len() <= 1 {
                    self.line_status &= !LSR_DATA_BIT;
                }
                // METRICS.uart.read_count.inc();
                self.in_buffer.pop_front().unwrap_or_default()
            }
            IER => self.interrupt_enable,
            IIR => {
                let v = self.interrupt_identification | IIR_FIFO_BITS;
                self.iir_reset();
                v
            }
            LCR => self.line_control,
            MCR => self.modem_control,
            LSR => self.line_status,
            MSR => self.modem_status,
            SCR => self.scratch,
            _ => 0,
        }
    }

    fn recv_bytes(&mut self) -> Result<usize, usize> {
        if let Some(input) = self.input.as_mut() {
            let mut out = [0u8; 32];
            return input.read(&mut out).and_then(|count| {
                if count > 0 {
                    self.raw_input(&out[..count])?;
                    Ok(count)
                } else {
                    Ok(0)
                }
            });
        }

        Ok(0)
    }

    fn raw_input(&mut self, data: &[u8]) -> Result<(), usize> {
        if !self.is_loop() {
            self.in_buffer.extend(data);
            self.recv_data_interrupt()?;
        }
        Ok(())
    }

    fn read(&mut self, offset: u64, data: &mut [u8]) {
        if data.len() != 1 {
            // METRICS.uart.missed_read_count.inc();
            return;
        }

        data[0] = self.handle_read(offset as u8);
    }

    fn write(&mut self, offset: u64, data: &[u8]) {
        if data.len() != 1 {
            // METRICS.uart.missed_write_count.inc();
            return;
        }
        if let Err(_e) = self.handle_write(offset as u8, data[0]) {
            // error!("Failed the write to serial: {}", e);
            // METRICS.uart.error_count.inc();
            assert!(false)
        }
    }
}

// impl Subscriber for Serial {
//     /// Handle events on the serial input fd.
//     fn process(&mut self, event: &EpollEvent, ev_mgr: &mut EventManager) {
//         let source = event.fd();

//         // We expect to be interested only in serial input.
//         let interest_list = self.interest_list();
//         if interest_list.len() != 1 {
//             warn!("Unexpected events/sources interest list.");
//             return;
//         }

//         // Safe to unwrap. Checked before if interest list has one element.
//         let supported_event = interest_list.first().unwrap();

//         // Check if the event source is the serial input.
//         if supported_event.fd() != source {
//             warn!("Unexpected event source: {}", source);
//             return;
//         }

//         // We expect to receive: `EventSet::IN`, `EventSet::HANG_UP` or
//         // `EventSet::ERROR`. To process all these events we just have to
//         // read from the serial input.
//         match self.recv_bytes() {
//             Ok(count) => {
//                 // Check if the serial input have to be unregistered.
//                 let event_set = event.event_set();
//                 let unregister_condition =
//                     event_set.contains(EventSet::ERROR) | event_set.contains(EventSet::HANG_UP);
//                 if count == 0 && unregister_condition {
//                     // Unregister the serial input source.
//                     match ev_mgr.unregister(supported_event.fd()) {
//                         Ok(_) => warn!("Detached the serial input due to peer error/close."),
//                         Err(e) => error!(
//                             "Peer is unreachable. Could not detach the serial input: {:?}",
//                             e
//                         ),
//                     }
//                 }
//             }
//             Err(e) => error!("error while reading stdin: {:?}", e),
//         }
//     }

//     /// Initial registration of pollable objects.
//     /// If serial input is present, register the serial input FD as readable.
//     fn interest_list(&self) -> Vec<EpollEvent> {
//         match &self.input {
//             Some(input) => vec![EpollEvent::new(EventSet::IN, input.as_raw_fd() as u64)],
//             None => vec![],
//         }
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use std::io;
//     use std::io::Write;
//     use std::os::unix::io::RawFd;
//     use std::sync::{Arc, Mutex};

//     use polly::event_manager::EventManager;

//     struct SharedBufferInternal {
//         read_buf: Vec<u8>,
//         write_buf: Vec<u8>,
//         evfd: EventFd,
//     }

//     #[derive(Clone)]
//     struct SharedBuffer {
//         internal: Arc<Mutex<SharedBufferInternal>>,
//     }

//     impl SharedBuffer {
//         fn new() -> SharedBuffer {
//             SharedBuffer {
//                 internal: Arc::new(Mutex::new(SharedBufferInternal {
//                     read_buf: Vec::new(),
//                     write_buf: Vec::new(),
//                     evfd: EventFd::new(libc::EFD_NONBLOCK).unwrap(),
//                 })),
//             }
//         }
//     }
//     impl io::Write for SharedBuffer {
//         fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
//             self.internal.lock().unwrap().write_buf.write(buf)
//         }
//         fn flush(&mut self) -> io::Result<()> {
//             self.internal.lock().unwrap().write_buf.flush()
//         }
//     }
//     impl io::Read for SharedBuffer {
//         fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
//             let count = self
//                 .internal
//                 .lock()
//                 .unwrap()
//                 .read_buf
//                 .as_slice()
//                 .read(buf)?;
//             // Need to clear what is read, to simulate consumed inflight bytes.
//             self.internal.lock().unwrap().read_buf.drain(0..count);
//             Ok(count)
//         }
//     }
//     impl AsRawFd for SharedBuffer {
//         fn as_raw_fd(&self) -> RawFd {
//             self.internal.lock().unwrap().evfd.as_raw_fd()
//         }
//     }
//     impl ReadableFd for SharedBuffer {}

//     static raw_input_buf: [u8; 3] = [b'a', b'b', b'c'];

//     #[test]
//     fn test_event_handling_no_in() {
//         let mut event_manager = EventManager::new().unwrap();

//         let intr_evt = EventFd::new(libc::EFD_NONBLOCK).unwrap();
//         let serial_out = SharedBuffer::new();

//         let mut serial = Serial::new_out(intr_evt, Box::new(serial_out));
//         // A serial without in does not have any events in the list.

//         assert!(serial.interest_list().is_empty());
//         // Even though there is no in or hangup, process should not panic. Call it to validate this.
//         let epoll_event = EpollEvent::new(EventSet::IN, 0);
//         serial.process(&epoll_event, &mut event_manager);
//     }

//     #[test]
//     fn test_event_handling_with_in() {
//         let mut event_manager = EventManager::new().unwrap();

//         let intr_evt = EventFd::new(libc::EFD_NONBLOCK).unwrap();
//         let serial_in_out = SharedBuffer::new();

//         let mut serial = Serial::new_in_out(
//             intr_evt.try_clone().unwrap(),
//             Box::new(serial_in_out.clone()),
//             Box::new(serial_in_out),
//         );
//         // Check that the interest list contains one event set.
//         assert_eq!(serial.interest_list().len(), 1);

//         // Process an invalid event type does not panic.
//         let invalid_event = EpollEvent::new(EventSet::OUT, intr_evt.as_raw_fd() as u64);
//         serial.process(&invalid_event, &mut event_manager);

//         // Process an event with a `RawFd` that does not correspond to `intr_evt` does not panic.
//         let invalid_event = EpollEvent::new(EventSet::IN, 0);
//         serial.process(&invalid_event, &mut event_manager);
//     }

//     #[test]
//     fn test_event_handling_err_and_hup() {
//         let mut event_manager = EventManager::new().unwrap();
//         let serial_in_out = SharedBuffer::new();
//         let mut serial = Serial::new_in_out(
//             EventFd::new(libc::EFD_NONBLOCK).unwrap(),
//             Box::new(serial_in_out.clone()),
//             Box::new(serial_in_out.clone()),
//         );

//         // Check that the interest list contains one event set.
//         let expected_medium_bytes = [b'a'; 32];
//         assert_eq!(serial.interest_list().len(), 1);
//         {
//             let mut guard = serial_in_out.internal.lock().unwrap();
//             // Write 33 bytes to the serial console. `IN` handling consumes 32 bytes from the
//             // inflight bytes. Add one more byte to be able to process it in a following
//             // processing round.
//             guard.read_buf.write_all(&expected_medium_bytes).unwrap();
//             guard.read_buf.write_all(&[b'a']).unwrap();
//         }

//         assert!(serial.in_buffer.is_empty());
//         let err_hup_ev = EpollEvent::new(
//             EventSet::ERROR | EventSet::HANG_UP,
//             serial_in_out.as_raw_fd() as u64,
//         );
//         // Process 32 bytes.
//         serial.process(&err_hup_ev, &mut event_manager);
//         // Process one more byte left.
//         serial.process(&err_hup_ev, &mut event_manager);
//         assert_eq!(serial.in_buffer.len(), expected_medium_bytes.len() + 1);
//         serial.in_buffer.clear();

//         // Process one more round of `EventSet::HANG_UP`.
//         // Check that the processing does not bring anything new to the serial
//         // `in_buffer`.
//         serial.process(&err_hup_ev, &mut event_manager);
//         assert!(serial.in_buffer.is_empty());
//     }

//     #[test]
//     fn test_serial_output() {
//         let intr_evt = EventFd::new(libc::EFD_NONBLOCK).unwrap();
//         let serial_out = SharedBuffer::new();

//         let mut serial = Serial::new_out(intr_evt, Box::new(serial_out.clone()));

//         // Invalid write of multiple chars at once.
//         serial.write(u64::from(DATA), &[b'x', b'y']);
//         // Valid one char at a time writes.
//         raw_input_buf
//             .iter()
//             .for_each(|&c| serial.write(u64::from(DATA), &[c]));
//         assert_eq!(
//             serial_out.internal.lock().unwrap().write_buf.as_slice(),
//             &raw_input_buf
//         );
//     }

//     #[test]
//     fn test_serial_recv_bytes() {
//         // Exercise bytes retrieval without any input.
//         {
//             let mut serial = Serial::new(EventFd::new(libc::EFD_NONBLOCK).unwrap(), None, None);

//             let count = serial.recv_bytes().unwrap();
//             assert!(count == 0);
//         }

//         // Prepare the input buffer and send bytes on the "medium".
//         {
//             let serial_in_out = SharedBuffer::new();
//             let mut serial = Serial::new_in_out(
//                 EventFd::new(libc::EFD_NONBLOCK).unwrap(),
//                 Box::new(serial_in_out.clone()),
//                 Box::new(serial_in_out.clone()),
//             );

//             // Serial `recv_bytes` consumes chunks of 32 bytes from the serial input.
//             // Write 33 bytes to assert on the consumption of only 32 bytes later on.
//             let expected_medium_bytes = [b'a'; 32];
//             {
//                 let mut guard = serial_in_out.internal.lock().unwrap();
//                 guard.read_buf.write_all(&expected_medium_bytes).unwrap();
//                 guard.read_buf.write_all(&[b'a']).unwrap();
//             }

//             let count = serial.recv_bytes().unwrap();
//             assert!(serial.in_buffer.len() == count);
//             assert!(serial.in_buffer == expected_medium_bytes);
//         }
//     }

//     #[test]
//     fn test_serial_input() {
//         let intr_evt = EventFd::new(libc::EFD_NONBLOCK).unwrap();
//         let serial_in_out = SharedBuffer::new();

//         let mut serial = Serial::new_in_out(
//             intr_evt.try_clone().unwrap(),
//             Box::new(serial_in_out.clone()),
//             Box::new(serial_in_out.clone()),
//         );

//         // Write 1 to the interrupt event fd, so that read doesn't block in case the event fd
//         // counter doesn't change (for 0 it blocks).
//         assert!(intr_evt.write(1).is_ok());
//         serial.write(u64::from(IER), &[IER_RECV_BIT]);

//         // Prepare the input buffer.
//         {
//             let mut guard = serial_in_out.internal.lock().unwrap();
//             guard.read_buf.write_all(&raw_input_buf).unwrap();
//             guard.evfd.write(1).unwrap();
//         }

//         let mut evmgr = EventManager::new().unwrap();
//         let serial_wrap = Arc::new(Mutex::new(serial));
//         evmgr.add_subscriber(serial_wrap.clone()).unwrap();

//         // Run the event handler which should drive serial input.
//         // There should be one event reported (which should have also handled serial input).
//         assert_eq!(evmgr.run_with_timeout(50).unwrap(), 1);

//         // Verify the serial raised an interrupt.
//         assert_eq!(intr_evt.read().unwrap(), 2);

//         let mut serial = serial_wrap.lock().unwrap();
//         let mut data = [0u8];
//         serial.read(u64::from(LSR), &mut data[..]);
//         assert_ne!(data[0] & LSR_DATA_BIT, 0);

//         // Verify reading the previously inputted buffer.
//         raw_input_buf.iter().for_each(|&c| {
//             serial.read(u64::from(DATA), &mut data[..]);
//             assert_eq!(data[0], c);
//         });
//     }

// }

fn main() {
    {
        // test_serial_raw_input
        let mut serial = Serial::new_out(EventFd {}, OutWrapper { output: Vec::new() });
        let raw_input_buf: [u8; 3] = [b'a', b'b', b'c'];

        // Write 1 to the interrupt event fd, so that read doesn't block in case the event fd
        // counter doesn't change (for 0 it blocks).
        // assert!(intr_evt.write(1).is_ok());
        serial.write(u64::from(IER), &[IER_RECV_BIT]);
        match serial.raw_input(&raw_input_buf) {
            Ok(_) => {}
            Err(_) => {}
        }

        // // Verify the serial raised an interrupt.
        // // assert_eq!(intr_evt.read().unwrap(), 2);

        // // Check if reading in a 2-length array doesn't have side effects.
        // let mut data = [0u8, 0u8];
        // serial.read(u64::from(DATA), &mut data[..]);
        // assert!(data == [0u8, 0u8]);

        // let mut data = [0u8];
        // serial.read(u64::from(LSR), &mut data[..]);
        // assert!(data[0] & LSR_DATA_BIT != 0);

        // // Verify reading the previously inputted buffer.
        // raw_input_buf.iter().for_each(|&c| {
        //     serial.read(u64::from(DATA), &mut data[..]);
        //     assert!(data[0] == c);
        // });

        // // Check if reading from the largest u8 offset returns 0.
        // serial.read(0xff, &mut data[..]);
        // assert!(data[0] == 0);
    }
}
