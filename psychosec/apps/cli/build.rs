fn main() {
    println!("cargo:rerun-if-changed=../../installer/psychosec.ico");
    println!("cargo:rerun-if-env-changed=PSYCHOSEC_API_URL");
    println!("cargo:rerun-if-env-changed=PSYCHOSEC_UPDATE_PUBKEY");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../installer/psychosec.ico");
        res.set("ProductName", "PSYCHOSEC");
        res.set("FileDescription", "PSYCHOSEC - Private Intelligence Terminal");
        res.set("CompanyName", "PSYCHOSEC");
        res.set("LegalCopyright", "Copyright (c) PSYCHOSEC");
        res.set("OriginalFilename", "Psychosec.exe");
        res.set("InternalName", "Psychosec");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed Windows resources: {e}");
        }
    }
}
