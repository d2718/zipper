mod cmds;

use std::io::{stdin, stdout, BufRead, Write};

use tokio::{
    runtime,
    sync::mpsc::Receiver,
};

use cmds::Cmd;

static SHELL: &str = "sh";
#[cfg(unix)]
static NEWLINE: &[u8] = &[10];
#[cfg(windows)]
static NEWLINE: &[u8] = &[13, 10];

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

async fn read_loop(mut outputs: Vec<Receiver<Vec<u8>>>) -> std::io::Result<()> {
    let mut stdout = stdout().lock();
    let mut done: bool = false;

    while !done {
        for rx in outputs.iter_mut() {
            match rx.recv().await {
                Some(v) => {
                    stdout.write_all(&v)?;
                    stdout.write_all(NEWLINE)?;
                }
                None => {
                    done = true;
                }
            }
        }
        stdout.flush()?;
    }

    Ok(())
}

fn main() -> Result<(), String> {
    let cmds = get_commands().unwrap();

    let rt = runtime::Builder::new_current_thread()
        .enable_io()
        .build()
        .map_err(|e| format!("unable to build runtime: {}", &e))?;

    rt.block_on(async {
        let outputs: Vec<_> = cmds
            .into_iter()
            .enumerate()
            .filter_map(|(n, cmd)| match cmd.spawn() {
                Ok(rx) => Some(rx),
                Err(e) => {
                    eprintln!("error spawning process {} {:?}: {}", &n, &cmd, &e);
                    None
                }
            })
            .collect();

        if let Err(e) = read_loop(outputs).await {
            eprintln!("{}", &e);
        }
    });

    Ok(())
}
