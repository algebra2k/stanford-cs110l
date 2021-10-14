use crate::debugger_command::DebuggerCommand;
use crate::dwarf_data::{DwarfData, Error as DwarfError};
use crate::inferior::Inferior;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::collections::HashMap;

#[derive(Clone)]
pub struct Breakpoint {
    pub breakpoint: usize,
    pub ori_byte: u8,
}

pub struct Debugger {
    target: String,
    history_path: String,
    readline: Editor<()>,
    inferior: Option<Inferior>,
    dwarf_data: Option<DwarfData>,
    breakpoint_hmap: HashMap<usize, Breakpoint>,
}

impl Debugger {
    /// Initializes the debugger.
    pub fn new(target: &str) -> Debugger {
        // TODO (milestone 3): initialize the DwarfData

        let history_path = format!("{}/.deet_history", std::env::var("HOME").unwrap());
        let mut readline = Editor::<()>::new();
        // Attempt to load history from ~/.deet_history if it exists
        let _ = readline.load_history(&history_path);

        let mut debugger = Debugger {
            target: target.to_string(),
            history_path,
            readline,
            inferior: None,
            dwarf_data: None,
            breakpoint_hmap: HashMap::new(),
        };

        debugger.dwarf_data = Some(debugger.load_dwarf_data(&target));
        debugger.dwarf_data.as_ref().unwrap().print();
        debugger
    }

    pub fn run(&mut self) {
        loop {
            match self.get_next_command() {
                DebuggerCommand::Run(args) => {
                    if self.inferior.is_some() {
                        match self.inferior.as_mut().unwrap().quit() {
                            Ok(_) => {
                                println!("The {} subprocess already running", self.target);
                                println!(
                                    "Quit running inferior (pid {})",
                                    self.inferior.as_ref().unwrap().pid().as_raw() as i32
                                );
                                self.inferior = None
                            }
                            Err(error) => match error.as_errno() {
                                // ignore ESRCH errno
                                Some(nix::errno::Errno::ESRCH) | None => (),
                                Some(errno) => {
                                    println!(
                                        "Quit running inferior (pid {}) error {}",
                                        self.inferior.as_ref().unwrap().pid().as_raw() as i32,
                                        errno
                                    )
                                }
                            },
                        }
                    }

                    if let Some(inferior) =
                        Inferior::new(&self.target, &args, &mut self.breakpoint_hmap)
                    {
                        // Create the inferior
                        self.inferior = Some(inferior);
                        match self
                            .inferior
                            .as_mut()
                            .unwrap()
                            .cont(&mut self.breakpoint_hmap)
                        {
                            Ok(status) => match status {
                                crate::inferior::Status::Stopped(signal, _rip) => {
                                    println!("Child stopped (signal {})", signal);
                                }
                                _ => (),
                            },
                            Err(error) => {
                                println!("run {} error: {}", self.target, error);
                            }
                        }
                    } else {
                        println!("Error starting subprocess");
                    }
                }
                DebuggerCommand::Cont => {
                    match self.inferior.as_mut() {
                        Some(inferior) => match inferior.cont(&mut self.breakpoint_hmap) {
                            Ok(status) => match status {
                                crate::inferior::Status::Stopped(signal, _rip) => {
                                    println!("Child stopped (signal {})", signal);
                                    // continue next instruction
                                }
                                _ => (),
                            },
                            Err(error) => match error.as_errno() {
                                // ignore ESRCH errno
                                Some(nix::errno::Errno::ESRCH) | None => {
                                    println!("no such running {} subprocess", self.target)
                                }
                                Some(errno) => {
                                    println!(
                                        "Error continue running {} subprocess {}",
                                        self.target, errno
                                    )
                                }
                            },
                        },
                        None => {
                            println!("Error continue running, not such subprocess")
                        }
                    }
                }
                DebuggerCommand::Backtrace => match self.inferior.as_ref() {
                    Some(inferior) => {
                        match inferior.print_backtrace(self.dwarf_data.as_ref().unwrap()) {
                            _ => (),
                        }
                    }
                    None => {
                        println!("Error continue running, not such subprocess")
                    }
                },
                DebuggerCommand::Break(args) => {
                    if args.len() == 0 {
                        println!("no set breakpoint");
                        continue;
                    }

                    let addr = match Debugger::parse_address(&args[0]) {
                        Some(addr) => addr,
                        None => {
                            println!("Set breakpoint at {} failed", &args[0]);
                            continue;
                        }
                    };
                    match self.inferior.as_mut() {
                        Some(inferior) => {
                            let ori_byte = match inferior.write_byte(addr, 0xcc) {
                                Ok(ori_byte) => ori_byte,
                                Err(error) => {
                                    println!("Error ({}) Set breakpoint {:#x}", error, addr);
                                    continue;
                                }
                            };

                            self.insert_breakpoint(addr, ori_byte);
                        }
                        None => {
                            self.insert_breakpoint(addr, 0xcc);
                            continue;
                        }
                    }
                }
                DebuggerCommand::Quit => {
                    // FIXME: inferior may be None
                    let inferior = self.inferior.as_mut().unwrap();
                    match inferior.quit() {
                        Ok(_) => {
                            println!(
                                "Killing running inferior (pid {})",
                                inferior.pid().as_raw() as i32
                            )
                        }
                        Err(error) => match error.as_errno() {
                            // ignore ESRCH errno
                            Some(nix::errno::Errno::ESRCH) | None => (),
                            Some(errno) => {
                                println!(
                                    "Killing running inferior (pid {}) error {}",
                                    inferior.pid().as_raw() as i32,
                                    errno
                                )
                            }
                        },
                    }
                    return;
                }
            }
        }
    }

    /// This function prompts the user to enter a command, and continues re-prompting until the user
    /// enters a valid command. It uses DebuggerCommand::from_tokens to do the command parsing.
    ///
    /// You don't need to read, understand, or modify this function.
    fn get_next_command(&mut self) -> DebuggerCommand {
        loop {
            // Print prompt and get next line of user input
            match self.readline.readline("(deet) ") {
                Err(ReadlineError::Interrupted) => {
                    // User pressed ctrl+c. We're going to ignore it
                    println!("Type \"quit\" to exit");
                }
                Err(ReadlineError::Eof) => {
                    // User pressed ctrl+d, which is the equivalent of "quit" for our purposes
                    return DebuggerCommand::Quit;
                }
                Err(err) => {
                    panic!("Unexpected I/O error: {:?}", err);
                }
                Ok(line) => {
                    if line.trim().len() == 0 {
                        continue;
                    }
                    self.readline.add_history_entry(line.as_str());
                    if let Err(err) = self.readline.save_history(&self.history_path) {
                        println!(
                            "Warning: failed to save history file at {}: {}",
                            self.history_path, err
                        );
                    }
                    let tokens: Vec<&str> = line.split_whitespace().collect();
                    if let Some(cmd) = DebuggerCommand::from_tokens(&tokens) {
                        return cmd;
                    } else {
                        println!("Unrecognized command.");
                    }
                }
            }
        }
    }

    /// load_dwarf_data load DwarfData from target
    fn load_dwarf_data(&self, target: &str) -> DwarfData {
        match DwarfData::from_file(target) {
            Ok(data) => data,
            Err(DwarfError::ErrorOpeningFile) => {
                println!("Could not open file {}", target);
                std::process::exit(1);
            }
            Err(DwarfError::DwarfFormatError(err)) => {
                println!("Could not debugging symbols from {}: {:?}", target, err);
                std::process::exit(1);
            }
        }
    }

    /// Parse input addr to unsize
    fn parse_address(addr: &str) -> Option<usize> {
        let addr_without_0x = if addr.to_lowercase().starts_with("0x") {
            &addr[2..]
        } else {
            &addr
        };
        usize::from_str_radix(addr_without_0x, 16).ok()
    }

    fn insert_breakpoint(&mut self, addr: usize, val: u8) {
        self.breakpoint_hmap.insert(
            addr,
            Breakpoint {
                breakpoint: addr,
                ori_byte: val,
            },
        );
        println!(
            "Set breakpoint {} at {:#x}",
            self.breakpoint_hmap.len() - 1,
            addr
        );
    }
}
