fn main() {
    #[cfg(target_os = "windows")]
    windres::Build::new().compile("verycross.rc").unwrap();
}
