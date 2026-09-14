pub mod download;
pub mod facet_shape;
pub mod global_args;
pub mod output;

use crate::cli::download::DownloadArgs;
use crate::cli::global_args::GlobalArgs;
use crate::cli::output::CliOutput;
use arbitrary::Arbitrary;
use eyre::Context;
use facet::Facet;
use figue::FigueBuiltins;
use figue::{self as args};
use teamy_cancellation::CancellationToken;

/// Downloads web links through the same PowerShell/yt-dlp flow used by the Windows Share target.
#[derive(Facet, Arbitrary, Debug)]
pub struct Cli {
    /// Global logging, cancellation, and output options.
    #[facet(flatten)]
    pub global_args: GlobalArgs,

    /// Standard CLI options such as help, version, and completions.
    #[facet(flatten)]
    #[arbitrary(default)]
    pub builtins: FigueBuiltins,

    /// The command to run.
    #[facet(args::subcommand)]
    pub command: Command,
}

impl PartialEq for Cli {
    fn eq(&self, other: &Self) -> bool {
        self.global_args == other.global_args && self.command == other.command
    }
}

impl Cli {
    /// # Errors
    ///
    /// Returns an error if the runtime cannot be created or the selected command fails.
    pub fn invoke(self, cancellation_token: CancellationToken) -> eyre::Result<CliOutput> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .wrap_err("failed to build Tokio runtime")?;
        runtime.block_on(async move { self.command.invoke(cancellation_token).await })
    }
}

/// Available console commands.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum Command {
    /// Open a downloader window for an HTTP or HTTPS URL.
    Download(DownloadArgs),
}

impl Command {
    /// # Errors
    ///
    /// Returns an error if cancellation was requested or the command fails.
    pub async fn invoke(self, cancellation_token: CancellationToken) -> eyre::Result<CliOutput> {
        cancellation_token.bail_if_cancelled()?;
        match self {
            Self::Download(args) => args.invoke().await,
        }
    }
}
