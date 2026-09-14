use crate::cli::output::CliOutput;
use crate::downloader::launch_downloader;
use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;

/// Open the yt-dlp downloader for a web URL.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct DownloadArgs {
    /// HTTP or HTTPS URL to download.
    #[facet(args::positional)]
    pub url: String,
}

#[derive(Facet, Debug)]
struct DownloadLaunch {
    url: String,
    process_id: u32,
}

impl DownloadArgs {
    /// # Errors
    ///
    /// Returns an error if the URL is invalid or the downloader cannot be started.
    #[expect(
        clippy::unused_async,
        reason = "command invoke methods share the async CLI dispatch shape"
    )]
    pub async fn invoke(self) -> eyre::Result<CliOutput> {
        let process_id = launch_downloader(&self.url)?;
        Ok(CliOutput::facet(DownloadLaunch {
            url: self.url,
            process_id,
        }))
    }
}
