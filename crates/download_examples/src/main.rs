pub fn main() {}

#[cfg(test)]
mod test {
    use downloader_rs::download_configuration::DownloadConfiguration;
    use downloader_rs::download_service::DownloadService;

    #[global_allocator]
    static ALLOC: dhat::Alloc = dhat::Alloc;

    #[test]
    fn test_download_example() {
        let profiler = dhat::Profiler::new_heap();

        let mut download_service = DownloadService::new()
            .set_worker_thread_count(4)
            .set_multi_thread(true);
        download_service.set_parallel_count(1);
        download_service.start_service();

        let url = "https://gh.con.sh/https://github.com/AaronFeng753/Waifu2x-Extension-GUI/releases/download/v2.21.12/Waifu2x-Extension-GUI-v2.21.12-Portable.7z";
        let config = DownloadConfiguration::new()
            .set_url(url)
            .set_file_path("temp/temp.7z")
            .set_chunk_download(true)
            .set_chunk_size(1024 * 1024 * 10)
            .build();
        let operation = download_service.add_downloader(config);

        while !operation.is_done() {
            println!("{}", operation.downloaded_size());
        }

        drop(operation);

        download_service.stop();

        while !download_service.is_finished() {}

        drop(download_service);
    }
}