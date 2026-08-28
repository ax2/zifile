#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../../packaging/msix/Assets/ZiFile.ico");
    println!("cargo:rerun-if-changed=application.manifest");

    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon("../../packaging/msix/Assets/ZiFile.ico")
            .set_manifest_file("application.manifest")
            .set("ProductName", "ZiFile")
            .set("FileDescription", "ZiFile Archive Studio")
            .set("CompanyName", "ZiCode")
            .set("LegalCopyright", "Copyright (c) ZiCode contributors")
            .set("OriginalFilename", "zifile-desktop.exe");
        resource.compile()?;
    }

    Ok(())
}
