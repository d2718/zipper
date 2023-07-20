use std::{
    io::{stdin, stdout, BufRead, Write},
    process::{Command, Stdio},
    sync::mpsc::{sync_channel, Receiver, SyncSender},
    thread::spawn,
};

use regex_chunker::ByteChunker;

fn get_commands() -> Result<Vec<String>, String> {
    let mut v: Vec<String> = Vec::new();
    for line in stdin().lock().lines().map(|r| r.unwrap()) {
        println!("{:?}", line.trim().as_bytes());
        if line.trim() == "" {
            break;
        } else {
            v.push(line);
        }
    }
    Ok(v)
}

fn main() {
    let cmds = get_commands().unwrap();

    let mut outputs: Vec<_> = cmds
        .into_iter()
        .map(|cmd| {
            let (tx, rx) = sync_channel::<Vec<u8>>(0);
            spawn(move || {
                let mut child = Command::new("sh")
                    .arg("-c")
                    .arg(&cmd)
                    .stdout(Stdio::piped())
                    .spawn()
                    .unwrap();
                let stdout = child.stdout.take().unwrap();
                let chunker = ByteChunker::new(stdout, r#"\r?\n"#).unwrap();
                for chunk in chunker {
                    let v = chunk.unwrap();
                    eprintln!("read line: {:?}", &String::from_utf8_lossy(&v));
                    tx.send(v).unwrap();
                }
                let _ = child.wait().unwrap();
            });
            rx
        })
        .collect();

    loop {
        for rx in outputs.iter_mut() {
            match rx.recv() {
                Ok(v) => {
                    eprintln!("main thread rec'd: {:?}", )
                    stdout().write_all(&v).unwrap(),
                },
                Err(_) => break,
            }
        }
    }
}
