use std::{
    error::Error,
    io::{BufRead, stdin, stdout, Write},
    process::Stdio,
};

use tokio::{
    process::Command,
    runtime,
    sync::mpsc::{channel, Sender, Receiver},
};
use tokio_stream::StreamExt;
use regex_chunker::stream::ByteChunker;

static SHELL: &str = "sh";
#[cfg(unix)]
static NEWLINE: &[u8] = &[10];
#[cfg(windows)]
static NEWLINE: &[u8] = &[13, 10];

fn get_commands() -> Result<Vec<String>, String> {
    let mut v: Vec<String> = Vec::new();
    for line in stdin().lock().lines().map(|r| r.unwrap()) {
        if line.trim() == "" {
            break;
        } else {
            v.push(line);
        }
    }
    Ok(v)
}

async fn wrapped_spawner(cmd: &str, tx: Sender<Vec<u8>>) -> Result<(), Box<dyn Error>> {
    let mut child = Command::new(SHELL)
        .arg("-c")
        .arg(cmd)
        .stdout(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().ok_or("no stdout handle!")?;

    let mut chunker = ByteChunker::new(stdout, r#"\r?\n"#)?;
    while let Some(chunk) = chunker.next().await {
        tx.send(chunk?).await?;
    }
    child.wait().await?;

    Ok(())

}

async fn spawn_process(cmd: String, tx: Sender<Vec<u8>>) {
    if let Err(e) = wrapped_spawner(&cmd, tx).await {
        eprintln!("error in child process {:?}: {}", &cmd, &e);
    }
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
                },
                None => { done = true; },
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
        .build().map_err(|e| format!("unable to build runtime: {}", &e))?;

    rt.block_on(async {

        let mut outputs: Vec<_> = cmds
            .into_iter()
            .map(|cmd| {
                let (tx, rx) = channel::<Vec<u8>>(1);
                tokio::spawn(async move {
                    spawn_process(cmd, tx).await;
                });
                rx
            })
            .collect();

        if let Err(e) = read_loop(outputs).await {
            eprintln!("{}", &e);
        }
    });

    Ok(())
}
