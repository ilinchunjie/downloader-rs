mod c;

#[cfg(test)]
mod test {
    use downloader_rs::download_configuration::DownloadConfiguration;
    use downloader_rs::download_service::{DownloadService};
    use downloader_rs::downloader::{Downloader};

    #[test]
    fn test_download_service() {
        let mut service = DownloadService::new();

        service.start_service();

        let url = "https://lan.sausage.xd.com/servers.txt".to_string();
        let config = DownloadConfiguration::new()
            .set_url(url)
            .set_download_in_memory()
            .build();
        let operation = service.add_downloader(config);

        while !operation.is_done() {
            println!("{}", operation.downloaded_size());
        }

        println!("{}", operation.text());

        service.stop();
    }
}