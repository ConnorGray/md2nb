//! Utilities for iteracting with a Wolfram Kernel child process.

use std::{path::PathBuf, process};

use wolfram_expr::Expr;
use wstp::{Link, Protocol};

#[derive(Debug)]
pub struct KernelProcess {
    process: process::Child,
    link: Link,
}

#[derive(Debug)]
pub struct Error(String);

impl From<wstp::Error> for Error {
    fn from(err: wstp::Error) -> Error {
        Error(format!("WSTP error: {err}"))
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error(format!("IO error: {err}"))
    }
}

impl KernelProcess {
    /// Launch a new Wolfram Kernel child process and establish a WSTP connection with it.
    ///
    /// See also the [wolfram-app-discovery](https://crates.io/crates/wolfram-app-discovery)
    /// crate, whose
    /// [`WolframApp::kernel_executable_path()`](https://docs.rs/wolfram-app-discovery/0.2.0/wolfram_app_discovery/struct.WolframApp.html#method.kernel_executable_path)
    /// method can be used to get the location of a
    /// [`WolframKernel`](https://reference.wolfram.com/language/ref/program/WolframKernel.html)
    /// executable suitable for use with this function.
    //
    // TODO: Would it be correct to describe this as essentially `LinkLaunch`? Also note
    //       that this doesn't actually use `-linkmode launch`.
    pub fn launch(path: &PathBuf) -> Result<KernelProcess, Error> {
        // FIXME: Make this a random string.
        const NAME: &str = "SHM_WK_LINK";

        let listener = std::thread::spawn(|| {
            // This will block until a connection is made.
            Link::listen(Protocol::SharedMemory, NAME)
        });

        let kernel_process = process::Command::new(path)
            .arg("-wstp")
            .arg("-linkprotocol")
            .arg("SharedMemory")
            .arg("-linkconnect")
            .arg("-linkname")
            .arg(NAME)
            .spawn()?;

        let link: Link = match listener.join() {
            Ok(result) => result?,
            Err(panic) => {
                return Err(Error(format!(
                    "unable to launch Wolfram Kernel: listening thread panicked: {:?}",
                    panic
                )))
            },
        };

        Ok(KernelProcess {
            process: kernel_process,
            link,
        })
    }

    /// Get the WSTP [`Link`] connection used to communicate with this Wolfram Kernel
    /// process.
    pub fn link(&mut self) -> &mut Link {
        let KernelProcess { process: _, link } = self;
        link
    }

    // TODO: Make this a `Link` method?
    pub fn put_eval_packet(&mut self, expr: &Expr) -> Result<(), Error> {
        self.link.put_function("System`EvaluatePacket", 1)?;
        self.link.put_expr(expr)?;
        self.link.end_packet()?;

        Ok(())
    }
}
