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

include!("../../rmc-prelude.rs");

fn main() {
    {
        // test_serial_modem()
        let mut serial = Serial::new_sink(EventFd {});
        let a: u8 = __nondet();
        let b: u8 = __nondet();
        let c: u8 = __nondet();

        serial.write(MCR as u64, &[MCR_LOOP_BIT as u8]);
        serial.write(DATA as u64, &[a]);
        serial.write(DATA as u64, &[b]);
        serial.write(DATA as u64, &[c]);

        let mut data = [0u8];
        serial.read(MSR as u64, &mut data[..]);
        assert!(data[0] == DEFAULT_MODEM_STATUS as u8);
        serial.read(MCR as u64, &mut data[..]);
        assert!(data[0] == MCR_LOOP_BIT as u8);
        serial.read(DATA as u64, &mut data[..]);
        assert!(data[0] == a);
        serial.read(DATA as u64, &mut data[..]);
        assert!(data[0] == b);
        serial.read(DATA as u64, &mut data[..]);
        assert!(data[0] == c);
    }

    // {
    //     // test_serial_data_len()
    //     const LEN: usize = 1;
    //     let mut serial = Serial::new_out(EventFd {}, OutWrapper::new());
    //     let a: u8 = __nondet();
    //     let b: u8 = __nondet();
    //     let c: u8 = __nondet();

    //     // let missed_writes_before = METRICS.uart.missed_write_count.count();
    //     // Trying to write data of length different than the one that we initialized the device with
    //     // should increase the `missed_write_count` metric.
    //     serial.write(u64::from(DATA), &[a, b]);
    //     assert!(serial.out.as_ref().unwrap().output.is_empty());
    //     // let missed_writes_after = METRICS.uart.missed_write_count.count();
    //     // assert!(missed_writes_before == missed_writes_after - 1);

    //     let data = [c; LEN];
    //     serial.write(u64::from(DATA), &data);
    //     assert!(&(serial.out.as_ref().unwrap().output)[..] == &data[..]);

    //     // When we write data that has the length used to initialize the device, the `missed_write_count`
    //     // metric stays the same.
    //     // assert!(missed_writes_before == missed_writes_after - 1);
    // }
}
