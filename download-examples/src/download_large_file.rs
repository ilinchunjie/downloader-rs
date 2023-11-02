use downloader_rs::download_configuration::DownloadConfiguration;
use downloader_rs::download_operation::DownloadOperation;
use downloader_rs::download_service::DownloadService;

pub fn start_download(service: &mut DownloadService) -> DownloadOperation {
    let url = "https://gh.con.sh/https://github.com/AaronFeng753/Waifu2x-Extension-GUI/releases/download/v2.21.12/Waifu2x-Extension-GUI-v2.21.12-Portable.7z".to_string();
    let config = DownloadConfiguration::new()
        .set_url(url)
        .set_file_path("temp/temp.7z".to_string())
        .set_chunk_download(true)
        .set_chunk_size(1024 * 1024 * 20)
        .create_dir()
        .build();
    let operation = service.add_downloader(config);
    return operation;
}