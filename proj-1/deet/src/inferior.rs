use crate::debugger::Breakpoint;
use crate::dwarf_data;
use nix::sys::ptrace;
use nix::sys::signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::collections::HashMap;
use std::mem::size_of;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};

pub enum Status {
    /// Indicates inferior stopped. Contains the signal that stopped the process, as well as the
    /// current instruction pointer that it is stopped at.
    Stopped(signal::Signal, usize),

    /// Indicates inferior exited normally. Contains the exit status code.
    Exited(i32),

    /// Indicates the inferior exited due to a signal. Contains the signal that killed the
    /// process.
    Signaled(signal::Signal),
}

pub struct Inferior {
    child: Child,
}

/// align address by word size
fn align_addr_to_word(addr: usize) -> usize {
    addr & (-(size_of::<usize>() as isize) as usize)
}

#[warn(dead_code)]
fn get_file_and_fn_name(
    data: &dwarf_data::DwarfData,
    instruction_ptr: usize,
) -> Option<(dwarf_data::Line, String)> {
    let fn_name = match data.get_function_from_addr(instruction_ptr) {
        Some(fn_name) => fn_name,
        None => return None,
    };

    let line = match data.get_line_from_addr(instruction_ptr) {
        Some(previous_line) => previous_line,
        None => return None,
    };

    Some((line, fn_name))
}

impl Inferior {
    /// Write a byte to a word
    pub fn write_byte(&mut self, addr: usize, val: u8) -> Result<u8, nix::Error> {
        let aligned_addr = align_addr_to_word(addr);
        let byte_offset = addr - aligned_addr;
        let word = ptrace::read(self.pid(), aligned_addr as ptrace::AddressType)? as u64;
        let orig_byte = (word >> 8 * byte_offset) & 0xff;
        let masked_word = word & !(0xff << 8 * byte_offset);
        let updated_word = masked_word | ((val as u64) << 8 * byte_offset);
        ptrace::write(
            self.pid(),
            aligned_addr as ptrace::AddressType,
            updated_word as *mut std::ffi::c_void,
        )?;
        Ok(orig_byte as u8)
    }
}

impl Inferior {
    /// Attempts to start a new inferior process. Returns Some(Inferior) if successful, or None if
    /// an error is encountered.
    pub fn new(
        target: &str,
        args: &Vec<String>,
        breakpoints_hmap: &mut HashMap<usize, Breakpoint>,
    ) -> Option<Inferior> {
        let mut cmd = Command::new(target);
        //exec ptrace before target exec
        unsafe {
            cmd.pre_exec(|| -> std::io::Result<()> {
                match ptrace::traceme() {
                    Ok(ok) => Ok(ok),
                    Err(error) => Err(std::io::Error::new(std::io::ErrorKind::Other, error)),
                }
            });
        }

        // spawn child process exec target
        let mut inferior = Inferior {
            child: cmd.args(args).spawn().ok()?,
        };

        // wait stopped signal
        match inferior.wait(Some(WaitPidFlag::WSTOPPED)) {
            Ok(status) => match status {
                Status::Stopped(signal, _rip) => match signal {
                    signal::Signal::SIGTRAP => {
                        for (addr, bp) in breakpoints_hmap.iter_mut() {
                            match inferior.write_byte(*addr, 0xcc) {
                                Ok(ori_byte) => {
                                    bp.ori_byte = ori_byte;
                                    continue;
                                }
                                Err(_error) => return None,
                            }
                        }
                        Some(inferior)
                    }
                    _ => None,
                },
                _ => None,
            },
            Err(_) => None,
        }
    }

    pub fn cont(
        &mut self,
        breakpoints_hmap: &mut HashMap<usize, Breakpoint>,
    ) -> Result<Status, nix::Error> {
        let mut regs = ptrace::getregs(self.pid())?;
        let rip = regs.rip as usize;

        if let Some(bp) = breakpoints_hmap.get(&(rip - 1)) {
            self.write_byte(bp.breakpoint, bp.ori_byte).ok();

            regs.rip = (rip - 1) as u64;
            ptrace::setregs(self.pid(), regs).ok();

            ptrace::step(self.pid(), None).ok();

            match self.wait(None).ok().unwrap() {
                Status::Exited(exit_code) => return Ok(Status::Exited(exit_code)),
                Status::Signaled(signal) => return Ok(Status::Signaled(signal)),
                Status::Stopped(_, _) => {
                    self.write_byte(rip - 1, 0xcc).ok();
                }
            }
        }
        match ptrace::cont(self.pid(), None) {
            Ok(_) => self.wait(None),
            Err(err) => Err(err),
        }
        // match ptrace::cont(self.pid(), None) {
        //     Ok(_) => self.wait_and_next(breakpoints_hmap),
        //     Err(error) => Err(error),
        // }
        // ptrace::cont(self.pid(), None)
        //     .and_then(|_| self.wait(None))
        //     .and_then(|status| match status {
        //         Status::Stopped(signal, rip) => match signal {
        //             signal::Signal::SIGTRAP => {
        //                 if let Some((line, _fn_name)) = get_file_and_fn_name(data, rip) {
        //                     println!(
        //                         "Stopped {:#x} at {}:{} (Signal {})",
        //                         rip, line.file, line.number, signal
        //                     )
        //                 }

        //                 let breakpoint = match breakpoints_hmap.get_mut(&(rip - 1)) {
        //                     None => {
        //                         println!("Failed get breakpoint in {:#x} instruction", rip - 1);
        //                         return Ok(status);
        //                     }
        //                     Some(breakpoint) => breakpoint,
        //                 };
        //                 println!("[cont] write_byte {:#x}, {:#x}", rip, breakpoint.ori_byte);
        //                 let _ori_byte = match self.write_byte(rip, breakpoint.ori_byte) {
        //                     Ok(ori_byte) => ori_byte,
        //                     Err(error) => return Err(error),
        //                 };

        //                 self.step_and_restore(breakpoints_hmap)
        //                 // .and_then(|_| ptrace::cont(self.pid(), signal::Signal::SIGCONT))
        //                 // .and_then(|_| self.wait(None))
        //                 // .and_then(|status| match status {
        //                 //     Status::Stopped(signal, rip) => match signal {
        //                 //         signal::Signal::SIGTRAP => {
        //                 //             if let Some((line, _fn_name)) = get_file_and_fn_name(data, rip) {
        //                 //                 // println!(
        //                 //                 //     "Stopped {:#x} at {}:{} (Signal {})",
        //                 //                 //     rip, line.file, line.number, signal
        //                 //                 // );
        //                 //                 let breakpoint = match breakpoints_hmap.get_mut(&(rip - 1)) {
        //                 //                     None => {
        //                 //                         println!("Failed get breakpoint in {:#x} instruction", rip - 1);
        //                 //                         return Ok(status);
        //                 //                     }
        //                 //                     Some(breakpoint) => breakpoint,
        //                 //                 };
        //                 //                 // println!(
        //                 //                 //     "get breakpoint {:#x}, val {:#x}",
        //                 //                 //     breakpoint.breakpoint, breakpoint.ori_byte
        //                 //                 // );
        //                 //                 let _ori_byte =
        //                 //                     match self.write_byte(breakpoint.breakpoint, breakpoint.ori_byte) {
        //                 //                         Ok(ori_byte) => ori_byte,
        //                 //                         Err(error) => return Err(error),
        //                 //                     };
        //                 //                 println!(
        //                 //                     "[cont] write_byte {:#x}, {:#x}",
        //                 //                     breakpoint.breakpoint, breakpoint.ori_byte
        //                 //                 );
        //                 //                 let mut regs = ptrace::getregs(self.pid()).unwrap();
        //                 //                 println!("regis.rip {:#x}, {:#x}", regs.rip, rip - 1);
        //                 //                 regs.rip = (rip - 1) as u64;
        //                 //                 ptrace::setregs(self.pid(), regs).unwrap();
        //                 //                 println!("set rip ok");
        //                 //             }
        //                 //             Ok(status)
        //                 //         }
        //                 //         _ => {
        //                 //             println!("oh no...");
        //                 //             Ok(status)
        //                 //         }
        //                 //     },
        //                 //     _ => Ok(status),
        //                 // })
        //             }
        //             _ => return Ok(status),
        //         },
        //         _ => return Ok(status),
        //     })
    }

    // fn wait_and_next(
    //     &mut self,
    //     breakpoints_hmap: &mut HashMap<usize, Breakpoint>,
    // ) -> Result<Status, nix::Error> {
    //     match self.wait(None) {
    //         Ok(status) => match status {
    //             Status::Stopped(signal, rip) => match signal {
    //                 signal::Signal::SIGTRAP => {
    //                     println!("Stopped {:#x}", rip - 1);
    //                     // replace 0xcc to ori_byte
    //                     let breakpoint = breakpoints_hmap.get_mut(&(rip - 1));
    //                     if breakpoint.is_none() {
    //                         panic!("Failed get breakpoint in {:#x} instruction", rip);
    //                     }

    //                     let breakpoint = breakpoint.unwrap();
    //                     println!(
    //                         "get breakpoint {:#x}, val {:#x}",
    //                         breakpoint.breakpoint, breakpoint.ori_byte
    //                     );
    //                     match self.write_byte(breakpoint.breakpoint, breakpoint.ori_byte) {
    //                         Ok(_ori_byte) => match self.step_and_restore(breakpoint.breakpoint) {
    //                             Ok(_) => match ptrace::cont(self.pid(), signal::Signal::SIGCONT) {
    //                                 Ok(_) => match self.wait(None) {
    //                                     Ok(status) => match status {
    //                                         Status::Stopped(signal, rip) => match signal {
    //                                             signal::Signal::SIGTRAP => {
    //                                                 self.write_byte(
    //                                                     breakpoint.breakpoint,
    //                                                     breakpoint.ori_byte,
    //                                                 )?;
    //                                                 let mut regs =
    //                                                     ptrace::getregs(self.pid()).unwrap();
    //                                                 regs.rip = (rip - 1) as u64;
    //                                                 println!("rip {:#x}", regs.rip);
    //                                                 ptrace::setregs(self.pid(), regs);
    //                                                 Ok(status)
    //                                             }
    //                                             _ => Ok(status),
    //                                         },
    //                                         _ => {
    //                                             println!("second wait other status");
    //                                             Ok(status)
    //                                         }
    //                                     },
    //                                     Err(error) => {
    //                                         print!("second wait error {}", error);
    //                                         Err(error)
    //                                     }
    //                                 },
    //                                 Err(error) => {
    //                                     println!("ptrace::cont error");
    //                                     Err(error)
    //                                 }
    //                             },
    //                             Err(error) => Err(error),
    //                         },
    //                         Err(error) => {
    //                             println!("write byte error");
    //                             Err(error)
    //                         }
    //                     }
    //                     // ptrace::step
    //                 }
    //                 _ => Ok(status),
    //             },
    //             _ => Ok(status),
    //         },
    //         Err(error) => Err(error),
    //     }
    // }

    // pub fn step_and_restore(
    //     &mut self,
    //     breakpoints_hmap: &mut HashMap<usize, Breakpoint>,
    // ) -> Result<Status, nix::Error> {
    //     ptrace::step(self.pid(), None)
    //         .and_then(|_| self.wait(None))
    //         .and_then(|status| match status {
    //             Status::Stopped(signal, rip) => match signal {
    //                 signal::Signal::SIGTRAP => {
    //                     println!("[step_and_restore] write_byte {:#x}, {:#x}", rip, 0xcc);
    //                     let ori_byte = match self.write_byte(rip, 0xcc) {
    //                         Ok(ori_byte) => ori_byte,
    //                         Err(error) => return Err(error),
    //                     };
    //                     breakpoints_hmap.insert(
    //                         rip,
    //                         Breakpoint {
    //                             breakpoint: rip,
    //                             ori_byte: ori_byte,
    //                         },
    //                     );
    //                     Ok(status)
    //                 }
    //                 _ => Ok(status),
    //             },
    //             _ => Ok(status),
    //         })

    // match ptrace::step(self.pid(), None) {
    //     Ok(_) => match self.wait(None) {
    //         Ok(status) => match status {
    //             Status::Stopped(signal, rip) => match signal {
    //                 signal::Signal::SIGTRAP => {
    //                     println!("[step_and_restore] Stopped {:#x} (Signal {})", rip, signal);
    //                     // restore breakpoint instruction
    //                     // FIXME: unwrap if None
    //                     self.write_byte(addr, 0xcc)?;

    //                     println!("[step_and_restore] write_byte error");
    //                     Ok(status)
    //                 }
    //                 _ => Ok(status),
    //             },
    //             _ => {
    //                 println!("step_and_restore other status");
    //                 Ok(status)
    //             }
    //         },
    //         Err(error) => Err(error),
    //     },
    //     Err(error) => Err(error),
    // }
    // }

    /// Returns the pid of this inferior.
    pub fn pid(&self) -> Pid {
        nix::unistd::Pid::from_raw(self.child.id() as i32)
    }

    /// Print the process backtrace.
    pub fn print_backtrace(&self, data: &dwarf_data::DwarfData) -> Result<(), nix::Error> {
        let regs = ptrace::getregs(self.pid())?;
        let mut line: dwarf_data::Line;
        let mut fn_name: String;
        let mut instruction_ptr = regs.rip as usize;
        let mut base_ptr = regs.rbp as usize;
        loop {
            // rbp address +8 get previous function address
            match data.get_function_from_addr(instruction_ptr) {
                None => break,
                Some(previous_fn_name) => fn_name = previous_fn_name,
            }

            // rip address +8 get previous function file info
            match data.get_line_from_addr(instruction_ptr) {
                None => break,
                Some(previous_line) => line = previous_line,
            }

            // print current function name
            println!("{} ({})", fn_name, line.file);

            // if function name is main, break
            if fn_name == "main" {
                break;
            }

            instruction_ptr =
                ptrace::read(self.pid(), (base_ptr + 8) as ptrace::AddressType)? as usize;
            base_ptr = ptrace::read(self.pid(), base_ptr as ptrace::AddressType)? as usize;
        }
        Ok(())
    }

    /// Calls waitpid on this inferior and returns a Status to indicate the state of the process
    /// after the waitpid call.
    pub fn wait(&self, options: Option<WaitPidFlag>) -> Result<Status, nix::Error> {
        Ok(match waitpid(self.pid(), options)? {
            WaitStatus::Exited(_pid, exit_code) => Status::Exited(exit_code),
            WaitStatus::Signaled(_pid, signal, _core_dumped) => Status::Signaled(signal),
            WaitStatus::Stopped(_pid, signal) => {
                let regs = ptrace::getregs(self.pid())?;
                Status::Stopped(signal, regs.rip as usize)
            }
            other => panic!("waitpid returned unexpected status: {:?}", other),
        })
    }

    /// Quit child process
    pub fn quit(&mut self) -> Result<Status, nix::Error> {
        match self.child.kill() {
            Ok(_) => Ok(Status::Exited(0)),
            Err(error) => Err(match error.raw_os_error() {
                Some(errno) => nix::Error::from_errno(nix::errno::Errno::from_i32(errno)),
                None => nix::Error::from_errno(nix::errno::Errno::UnknownErrno),
            }),
        }
    }
}
