fn main() -> eyre::Result<()> {
    #[cfg(windows)]
    if std::env::args_os().nth(1).is_none()
        && share_to_ytdlp::share_target::try_handle_activation()?
    {
        return Ok(());
    }

    share_to_ytdlp::main()
}
