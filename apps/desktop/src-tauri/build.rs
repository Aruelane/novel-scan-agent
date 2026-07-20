fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&["import_capabilities", "import_novel_bytes", "rule_pack_summary"]),
    ))
    .expect("failed to build Tauri application metadata");
}
