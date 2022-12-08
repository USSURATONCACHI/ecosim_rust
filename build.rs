use std::io;
#[cfg(windows)] use winres::WindowsResource;

fn main() -> io::Result<()> {
	#[cfg(windows)] {
		println!("cargo:rerun-if-changed=icon.ico");
		WindowsResource::new()
			// This path can be absolute, or relative to your crate root.
			.set_icon("icon.ico")
			.compile()?;
	}
	Ok(())
}
