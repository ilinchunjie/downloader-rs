mod c;

#[cfg(test)]
mod test {
    use downloader_rs::download_configuration::DownloadConfiguration;
    use downloader_rs::download_operation::DownloadOperation;
    use downloader_rs::download_service::{DownloadService};
    use downloader_rs::downloader::{Downloader};

    #[test]
    fn test_download_service() {
        let mut service = DownloadService::new();

        service.start_service();

        let url = "https://n17x06.xdcdn.net/media/SS6_CG_Weather_Kingdom.mp4".to_string();
        let config = DownloadConfiguration::new()
            .set_url(url)
            .set_file_path("temp/temp.mp4".to_string())
            .set_chunk_download(true)
            .set_chunk_size(1024 * 1024 * 30)
            .create_dir()
            .build();
        let operation = service.add_downloader(config);

        while !operation.is_done() {
            println!("{}", operation.downloaded_size());
        }

        service.stop();
    }
}