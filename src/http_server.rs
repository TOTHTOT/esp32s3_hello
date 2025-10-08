use crate::board::BoardEsp32State;
use embedded_svc::http::Method;
use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::io::Write;
use std::fs;
use std::sync::{Arc, Mutex};

const FS_MOUNT_POINT: &str = "/fat";

pub struct HttpServer<'d> {
    server: EspHttpServer<'d>,
}

impl<'d> HttpServer<'d> {
    // 开启http服务
    pub fn new(board: Arc<Mutex<BoardEsp32State>>) -> anyhow::Result<Self, anyhow::Error> {
        let mut server = EspHttpServer::new(&esp_idf_svc::http::server::Configuration::default())?;
        let board_state = board.lock().expect("Failed to lock board mutex");

        Self::http_server_add_page(&mut server, "/", Self::index_html())?;
        Self::http_server_add_page(
            &mut server,
            "/temp",
            Self::temperature(board_state.current_mcu_temperature),
        )?;
        log::info!("http server running");
        let mut httpserver = Self { server };
        httpserver.file_list(FS_MOUNT_POINT.to_string())?;

        Ok(httpserver)
    }
    /// 添加一个页面
    fn http_server_add_page(
        server: &mut EspHttpServer,
        url: &str,
        html: String,
    ) -> anyhow::Result<()> {
        server.fn_handler(url, Method::Get, move |request| {
            let mut response = match request.into_ok_response() {
                Ok(response) => response,
                Err(err) => {
                    log::warn!("Failed to read response: {:?}", err);
                    return Err(());
                }
            };
            response.write_all(html.as_bytes()).unwrap();
            Ok(())
        })?;
        Ok(())
    }

    /// 列出 fat 目录下的文件
    fn file_list(&mut self, path: String) -> anyhow::Result<()> {
        self.server.fn_handler("/files", Method::Get, move |req| {
            let entries = fs::read_dir(path.clone())?
                .filter_map(|e| e.ok())
                .map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    let is_dir = e.file_type().ok().map(|t| t.is_dir()).unwrap_or(false);
                    format!("{} ({})", name, if is_dir { "dir" } else { "file" })
                })
                .collect::<Vec<_>>()
                .join("\n");

            let mut resp = req.into_ok_response()?;
            resp.write_all(entries.as_bytes())?;
            // Ok(())
            Ok::<(), anyhow::Error>(())
        })?;
        Ok(())
    }
    fn templated(content: impl AsRef<str>) -> String {
        format!(
            r#"
    <!DOCTYPE html>
    <html>
        <head>
            <meta charset="utf-8">
            <title>esp-rs web server</title>
        </head>
        <body>
            {}
        </body>
    </html>
    "#,
            content.as_ref()
        )
    }

    fn index_html() -> String {
        Self::templated("Hello from ESP32-S3!")
    }

    fn temperature(val: f32) -> String {
        Self::templated(format!("mcu temperature: {}", val))
    }
}
