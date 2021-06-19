use nix::libc::_exit;
use nix::sys::signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{alarm, chdir, execvp, fork, getpid, setsid, ForkResult, Pid};
use std::ffi::CString;

const TIMEO: u32 = 15;

struct Sigmap {
    sig: signal::Signal,
    handler: unsafe fn(set: &signal::SigSet),
}

fn main() {
    let rcinitcmd: [CString; 1] =
        [CString::new("/home/nishal/nishal_init.test").expect("Failed CString")];
    let set: signal::SigSet;
    let sigmap: [Sigmap; 4] = [
        Sigmap {
            sig: signal::Signal::SIGUSR1,
            handler: sigpoweroff,
        },
        Sigmap {
            sig: signal::Signal::SIGCHLD,
            handler: sigreap,
        },
        Sigmap {
            sig: signal::Signal::SIGALRM,
            handler: sigreap,
        },
        Sigmap {
            sig: signal::Signal::SIGINT,
            handler: sigreboot,
        },
    ];

    if getpid() != Pid::from_raw(1) {
        println!("PID: {}", getpid());
        // std::process::exit(1);
    }

    match chdir("/") {
        Ok(_) => println!("Change dir worked"),
        Err(e) => println!("Change dir failed: {}", e),
    }

    set = signal::SigSet::all();

    match signal::sigprocmask(signal::SigmaskHow::SIG_BLOCK, Some(&set), None) {
        Ok(_) => println!("sigprocmask SIG_BLOCK worked"),
        Err(e) => println!("sigprocmask SIG_BLOCK failed {}", e),
    }

    unsafe {
        spawn(&rcinitcmd, &set);
    }

    loop {
        alarm::set(TIMEO);
        match signal::SigSet::wait(&set) {
            Ok(sig) => {
                println!("sigwait successful");
                for i in sigmap.iter() {
                    if i.sig == sig {
                        unsafe {
                            (i.handler)(&set);
                        }
                        break;
                    }
                }
            }
            Err(e) => println!("sigwait failed {}", e),
        }
    }
}

unsafe fn sigpoweroff(set: &signal::SigSet) {
    let rcpoweroffcmd: [CString; 2] = [
        CString::new("/home/nishal/nishal_shutdown.test").expect("Failed CString"),
        CString::new("poweroff").expect("Failed CString"),
    ];

    println!("rcpoweroffcmd");
    spawn(&rcpoweroffcmd, set);
}

fn sigreap(_set: &signal::SigSet) {
    println!("sigreap");

    loop {
        match waitpid(Pid::from_raw(-1), Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::StillAlive) => {
                println!("[main] Child is still alive, do my own stuff while waiting.");
            }
            Ok(status) => {
                println!("[main] Child exited with status {:?}.", status);
                break;
            }
            Err(err) => panic!("[main] waitpid() failed: {}", err),
        }
    }
    alarm::set(TIMEO);
}

unsafe fn sigreboot(set: &signal::SigSet) {
    let rcrebootcmd: [CString; 2] = [
        CString::new("/home/nishal/nishal_shutdown.test").expect("Failed CString"),
        CString::new("reboot").expect("Failed CString"),
    ];

    println!("rcrebootcmd");
    spawn(&rcrebootcmd, set);
}

unsafe fn spawn(argv: &[CString], set: &signal::SigSet) {
    match fork() {
        Ok(ForkResult::Parent { child }) => {
            println!(
                "Continuing execution in parent process, new child has pid: {}",
                child
            );
        }
        Ok(ForkResult::Child) => {
            println!("I'm a new child process");
            match signal::sigprocmask(signal::SigmaskHow::SIG_UNBLOCK, Some(set), None) {
                Ok(_) => {
                    println!("sigprocmask SIG_UNBLOCK worked");
                    match setsid() {
                        Ok(sid) => println!("setsid worked {}", sid),
                        Err(e) => println!("setsid failed {}", e),
                    }
                    match execvp(&argv[0], argv) {
                        Ok(_) => println!("execvp worked"),
                        Err(e) => {
                            println!("execvp failed {}", e);
                            _exit(1);
                        }
                    }
                }
                Err(e) => println!("sigprocmask SIG_UNBLOCK failed {}", e),
            }
        }
        Err(_) => println!("Fork failed"),
    }
}