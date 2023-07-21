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

// Find the position (of the start of) the sublsice `needle` in the
// slice `haystack`. Returns `None` if not present.
/* fn find_subslice<T: PartialEq>(haystack: &[T], needle: &[T]) -> Option<usize> {
    if needle.len() > haystack.len() {
        return None;
    }

    for (n, w) in haystack.windows(needle.len()).enumerate() {
        if w == needle {
            return Some(n);
        }
    }

    None
} */

/// Optons to specify how the output of a specific subcommand is treated.
#[derive(Debug, Parser)]
pub struct CmdOpts {
    #[arg(short, long)]
    delimiter: Option<String>,
    #[arg(long)]
    max: Option<usize>,
}

impl Default for CmdOpts {
    fn default() -> Self {
        CmdOpts {
            delimiter: None,
            max: None,
        }
    }
}

impl CmdOpts {
    fn parse(snip: &str) -> Result<CmdOpts, String> {
        let chunks =
            shlex::split(snip).ok_or_else(|| "unable to parse option snippet".to_string())?;
        let d = String::new();
        CmdOpts::try_parse_from([d].into_iter().chain(chunks.into_iter()))
            .map_err(|e| format!("{}", &e))
    }
}

#[derive(Debug)]
pub struct Cmd {
    opts: CmdOpts,
    exec: String,
}

async fn read_loop<A>(
    mut chunker: ByteChunker<A>,
    tx: Sender<Vec<u8>>,
) -> Result<(), Box<dyn Error>>
where
    A: AsyncRead + Unpin,
{
    while let Some(chunk) = chunker.next().await {
        tx.send(chunk?).await?;
    }
    Ok(())
}

impl Cmd {
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

    pub fn spawn(&self) -> Result<Receiver<Vec<u8>>, Box<dyn Error>> {
        let mut child = Command::new(SHELL)
            .arg("-c")
            .arg(&self.exec)
            .stdout(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().ok_or("no stdout handle!")?;

        let chunker = ByteChunker::new(stdout, self.opts.delimiter.as_deref().unwrap_or(DELIMITER))?;
        let (tx, rx) = channel::<Vec<u8>>(1);
        let self_debug_string = format!("{:?}", &self);

        tokio::spawn(async move {
            if let Err(e) = read_loop(chunker, tx).await {
                eprintln!("error reading output of {:?}: {}", &self_debug_string, &e);
            }
            if let Err(e) = child.wait().await {
                eprintln!(
                    "error waiting for process {:?} to finish: {}",
                    &self_debug_string, &e
                );
            }
        });

        Ok(rx)
    }
}
