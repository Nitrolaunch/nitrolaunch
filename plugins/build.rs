use std::path::PathBuf;
use std::{fs::File, path::Path};

use zip::{CompressionMethod, ZipWriter, write::FileOptions};

macro_rules! add_file {
	($zip:expr, $path:expr) => {
		let path2 = Path::new("../docs").join($path);
		if !path2.exists() {
			panic!("Doc file {path2:?} does not exist");
		}
		$zip.start_file(
			$path,
			FileOptions::<()>::default().compression_method(CompressionMethod::Deflated),
		)
		.unwrap();
		std::io::copy(&mut File::open(&path2).unwrap(), $zip).unwrap();
		println!("cargo::rerun-if-changed={path2:?}");
	};
}

fn main() {
	let out = File::create("./zipped_docs.zip").unwrap();
	let mut zip = ZipWriter::new(out);

	fn walk(dir: &Path, zip: &mut ZipWriter<File>) {
		for entry in std::fs::read_dir(dir).unwrap() {
			let entry = entry.unwrap();
			let path = entry.path();
			if path.is_dir() {
				zip.add_directory(
					path.strip_prefix("../docs").unwrap().to_string_lossy(),
					FileOptions::<()>::default(),
				)
				.unwrap();
				walk(&path, zip);
			} else if path.is_file() && path.extension().map(|s| s == "md").unwrap_or(false) {
				add_file!(
					zip,
					path.strip_prefix("../docs")
						.unwrap()
						.to_string_lossy()
						.to_string()
				);
			}
		}
	}

	let docs = PathBuf::from("../docs");
	walk(&docs, &mut zip);

	zip.finish().unwrap();
}
