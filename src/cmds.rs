/**
Command and option parsing.
*/
use std::{error::Error, process::Stdio};

use clap::Parser;
use regex_chunker::stream::ByteChunker;
use tokio::{
    io::AsyncRead,
    process::Command,
    sync::mpsc::{channel, Receiver, Sender},
};
use tokio_stream::StreamExt;

static SHELL: &str = "sh";
static SEPARATOR: &str = " || ";
static DELIMITER: &str = r#"\r?\n"#;

/**
Optons to specify how the output of a specific subcommand is treated.

These values are derived from the pre-command portion of a command
line, if present.
*/
#[derive(Clone, Debug, Parser)]
pub struct CmdOpts {
    /// regex to use as a delimiter instead of a newline
    #[arg(short, long)]
    delimiter: Option<String>,
    /// skip this many lines before echoing output
    #[arg(short, long)]
    skip: Option<usize>,
    /// only output this many lines, then stop
    #[arg(short, long)]
    take: Option<usize>,
}

impl Default for CmdOpts {
    fn default() -> Self {
        CmdOpts {
            delimiter: None,
            skip: None,
            take: None,
        }
    }
}

impl CmdOpts {
    fn parse(snip: &str) -> Result<CmdOpts, String> {
        let chunks =
            shlex::split(snip).ok_or_else(|| "unable to parse option snippet".to_string())?;
        // shlex is for parsing command lines, so it assumes that the first
        // chunk is the name of the executable. In this context, we don't
        // have an executable, so we have to start with a dummy chunk.
        let dummy = String::new();
        CmdOpts::try_parse_from([dummy].into_iter().chain(chunks.into_iter()))
            .map_err(|e| format!("{}", &e))
    }
}

/**
Specify to the task spawner the tokio threading model to use.

This is used to control the behavior of [`Cmd::spawn`].
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Threading {
    /// Spawn tasks using `tokio::task::spawn_local`
    Local,
    /// Spawn tasks using `tokio::spawn`
    Multi,
}

/**
All the information required to run one of the commands whose output
will be interleaved.
*/
#[derive(Debug)]
pub struct Cmd {
    /// Information from the line prefix options.
    opts: CmdOpts,
    /// Command string to pass to the shell.
    exec: String,
}

/// Read the output of a command and pass it chunk by chunk down
/// the provided channel.
async fn read_loop<A>(
    mut chunker: ByteChunker<A>,
    opts: CmdOpts,
    tx: Sender<Vec<u8>>,
) -> Result<(), Box<dyn Error>>
where
    A: AsyncRead + Unpin,
{
    if let Some(n) = opts.skip {
        for _ in 0..n {
            if let None = chunker.next().await {
                return Ok(());
            }
        }
    }
    if let Some(n) = opts.take {
        let mut chunker = chunker.take(n);
        while let Some(chunk) = chunker.next().await {
            tx.send(chunk?).await?;
        }
    } else {
        while let Some(chunk) = chunker.next().await {
            tx.send(chunk?).await?;
        }
    }
    Ok(())
}

impl Cmd {
    /// Read a line from stdin and parse it into a command to run.
    pub fn from_line(line: &str) -> Result<Cmd, String> {
        // This line is pretty gross, but it works. We have to supply
        // the turbofish to collect into a Vec, then the slicing index
        // to get a slice.
        let (opts, exec) = match line.splitn(2, SEPARATOR).collect::<Vec<_>>()[..] {
            [opts, exec] => {
                let opts = CmdOpts::parse(opts)
                    .map_err(|e| format!("error reading input command {:?}: {}", line, &e))?;
                (opts, String::from(exec))
            }
            [exec] => (CmdOpts::default(), String::from(exec)),
            _ => return Err(format!("unable to decipher command {:?}", line)),
        };

        Ok(Cmd { opts, exec })
    }

    /// Spawn the represented command under the given `Threading` mode.
    pub fn spawn(&self, t: Threading) -> Result<Receiver<Vec<u8>>, Box<dyn Error>> {
        let mut child = Command::new(SHELL)
            .arg("-c")
            .arg(&self.exec)
            .stdout(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().ok_or("no stdout handle!")?;

        let chunker =
            ByteChunker::new(stdout, self.opts.delimiter.as_deref().unwrap_or(DELIMITER))?;
        let (tx, rx) = channel::<Vec<u8>>(1);
        let self_debug_string = format!("{:?}", &self);
        let opts = self.opts.clone();

        match t {
            Threading::Local => {
                tokio::task::spawn_local(async move {
                    if let Err(e) = read_loop(chunker, opts, tx).await {
                        eprintln!("error reading output of {:?}: {}", &self_debug_string, &e);
                    }
                    if let Err(e) = child.wait().await {
                        eprintln!(
                            "error waiting for process {:?} to finish: {}",
                            &self_debug_string, &e
                        );
                    }
                });
            }
            Threading::Multi => {
                tokio::spawn(async move {
                    if let Err(e) = read_loop(chunker, opts, tx).await {
                        eprintln!("error reading output of {:?}: {}", &self_debug_string, &e);
                    }
                    if let Err(e) = child.wait().await {
                        eprintln!(
                            "error waiting for process {:?} to finish: {}",
                            &self_debug_string, &e
                        );
                    }
                });
            },
        }
        Ok(rx)
    }
}
