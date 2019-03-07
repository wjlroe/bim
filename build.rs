fn main() {
    #[cfg(target_os = "windows")]
    {
        use std::path::Path;

        let mut res = winres::WindowsResource::new();
        let icon_path = "bim.ico";
        if Path::new(icon_path).exists() {
            res.set_icon(icon_path);
        }
        res.compile().unwrap();
    }
}
