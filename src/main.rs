mod cmds;

use std::{error::Error, io::{stdin, stdout, BufRead, Write}};

use clap::{Parser, ValueEnum};
use tokio::{runtime, sync::mpsc::Receiver, task::LocalSet};

use cmds::{Cmd, Threading};

#[cfg(unix)]
static NEWLINE: &[u8] = &[10];
#[cfg(windows)]
static NEWLINE: &[u8] = &[13, 10];

/// Controls how zipper behaves when commands terminate after
/// different amounts of output.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
enum Finished {
    /// Stop when first command terminates.
    #[default]
    Terminate,
    /// Ignore terminated commands.
    Ignore,
    /// Insert blank lines for commands that have terminated.
    Blank,
}

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Cfg {
    /// Specify behavior on command termination.
    #[arg(short, long, value_enum, default_value_t = Finished::Terminate)]
    exit: Finished,
    /// Use more threads.
    #[arg(short, long, default_value_t = false)]
    threads: bool,
}

/**
Read from stdin until EOF or a blank line, and return a Vec of
`Cmd` structs to run.
*/
fn get_commands() -> Result<Vec<Cmd>, String> {
    let mut v: Vec<Cmd> = Vec::new();
    for (n, line_res) in stdin().lock().lines().enumerate() {
        let line = line_res.map_err(|e| format!("error reading line {} from stdin: {}", &n, &e))?;

        if line.trim() == "" {
            break;
        } else {
            let cmd = Cmd::from_line(&line)
                .map_err(|e| format!("error parsing command from line {}: {}", &n, &e))?;
            v.push(cmd);
        }
    }

    Ok(v)
}

/**
Repeatedly iterate through the output channels of the commands,
writing lines from each to stdout.
*/
async fn read_loop(mut outputs: Vec<Receiver<Vec<u8>>>, finished: Finished) -> std::io::Result<()> {
    let mut buff: Vec<u8> = Vec::new();
    let mut dones: Vec<bool> = vec![false; outputs.len()];
    let exit_condition: Vec<bool> = vec![true; outputs.len()];

    'writeloop: while &dones != &exit_condition {
        for (n, rx) in outputs.iter_mut().enumerate() {
            match rx.recv().await {
                Some(v) => {
                    buff.write_all(&v)?;
                    buff.write_all(NEWLINE)?;
                }
                None => {
                    dones[n] = true;
                    match finished {
                        Finished::Terminate => break 'writeloop,
                        Finished::Ignore => { /* do nothing */ }
                        Finished::Blank => {
                            buff.write_all(NEWLINE)?;
                        }
                    }
                }
            }
        }
        let mut stdout = stdout().lock();
        stdout.write_all(&buff)?;
        stdout.flush()?;
        buff.clear();
    }

    Ok(())
}

/// Spawn commands and interleave their output using tokio's default
/// (multh-threaded) task scheduler.
fn run_threaded(cmds: Vec<Cmd>, cfg: Cfg) -> Result<(), Box<dyn Error>> {
    let rt = runtime::Builder::new_multi_thread()
        .enable_io()
        .build()?;
    
    rt.block_on(async {
        let outputs: Vec<_> = cmds.into_iter()
            .enumerate()
            .filter_map(|(n, cmd)| match cmd.spawn(Threading::Multi) {
                Ok(rx) => Some(rx),
                Err(e) => {
                    eprintln!("error spawning process {} {:?}: {}", &n, &cmd, &e);
                    None
                },
            }).collect();

        read_loop(outputs, cfg.exit).await  
    }).map_err(|e| e.into())
}

/// Spawn commands and interleave their output using tokio's single-threaded
/// task scheduler. This does not guarantee strict single-threaded behavior.
/// Even if it did, each command still needs to run in its own process.
fn run_local(cmds: Vec<Cmd>, cfg: Cfg) -> Result<(), Box<dyn Error>> {
    let rt = runtime::Builder::new_current_thread()
        .enable_io()
        .build()?;
    
    rt.block_on(async {
        let local = LocalSet::new();

        local.run_until(async move {
            let outputs: Vec<_> = cmds.into_iter()
                .enumerate()
                .filter_map(|(n, cmd)| match cmd.spawn(Threading::Local) {
                    Ok(rx) => Some(rx),
                    Err(e) => {
                        eprintln!("error spawning process {} {:?}: {}", &n, &cmd, &e);
                        None
                    },
                }).collect();

            read_loop(outputs, cfg.exit).await
        }).await
    }).map_err(|e| e.into())
}

fn main() -> Result<(), Box<dyn Error>> {
    let cfg = Cfg::parse();
    let cmds = get_commands().unwrap();

    if cfg.threads {
        run_threaded(cmds, cfg)
    } else {
        run_local(cmds, cfg)
    }
}
