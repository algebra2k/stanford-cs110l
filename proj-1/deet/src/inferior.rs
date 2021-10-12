use nix::sys::ptrace;
use nix::sys::signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};
use crate::dwarf_data;

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

impl Inferior {
    /// Attempts to start a new inferior process. Returns Some(Inferior) if successful, or None if
    /// an error is encountered.
    pub fn new(target: &str, args: &Vec<String>) -> Option<Inferior> {
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
        let inferior = Inferior {
            child: cmd.args(args).spawn().ok()?,
        };

        // wait stopped signal
        match inferior.wait(Some(WaitPidFlag::WSTOPPED)) {
            Ok(status) => match status {
                Status::Stopped(_signal, _rip) => Some(inferior),
                _ => None,
            },
            Err(_) => None,
        }
    }

    pub fn cont(&self) -> Result<Status, nix::Error> {
        match ptrace::cont(self.pid(), None) {
            Ok(_) => self.wait(None),
            Err(error) => Err(error),
        }
    }

    /// Returns the pid of this inferior.
    pub fn pid(&self) -> Pid {
        nix::unistd::Pid::from_raw(self.child.id() as i32)
    }

    /// Print the process backtrace.
    pub fn print_backtrace(&self, data: &dwarf_data::DwarfData) -> Result<(), nix::Error> {
        let regs = ptrace::getregs(self.pid())?;
        let mut line: dwarf_data::Line ;
        let mut fn_name: String ;
        let mut instruction_ptr = regs.rip as usize;
        let mut base_ptr = regs.rbp as usize;
        loop {
              // rbp address +8 get previous function address
              match data.get_function_from_addr(instruction_ptr) {
                None => break,
                Some(previous_fn_name) => fn_name = previous_fn_name,
            }

            // rip address +8 get previous function file info
            match data.get_line_from_addr(instruction_ptr ){
                None => break,
                Some(previous_line) => line = previous_line,
            }

            // print current function name
            println!("{} ({})", fn_name, line.file);

            // if function name is main, break
            if fn_name == "main" {
                break;
            }
            
            instruction_ptr = ptrace::read(self.pid(), (base_ptr + 8) as ptrace::AddressType)? as usize;
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
                Some(errno) => nix::Error::from_errno( nix::errno::Errno::from_i32(errno)),
                None => nix::Error::from_errno(nix::errno::Errno::UnknownErrno),
            }),
        }
    }
}
