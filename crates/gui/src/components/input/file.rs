use std::path::PathBuf;

use rfd::FileDialog;

use crate::prelude::*;

#[derive(PartialEq)]
pub struct FileSelector {
	path: State<Option<PathBuf>>,
	save: bool,
	file: bool,
}

impl FileSelector {
	pub fn select(path: State<Option<PathBuf>>) -> Self {
		Self {
			path,
			save: false,
			file: true,
		}
	}

	pub fn save(path: State<Option<PathBuf>>) -> Self {
		Self {
			path,
			save: true,
			file: true,
		}
	}
}

impl Component for FileSelector {
	fn render(&self) -> impl IntoElement {
		let theme = use_theme();

		let path = self.path.clone();
		let preview = if let Some(path) = path.read().as_ref() {
			path.to_string_lossy().to_string()
		} else {
			"Select file".into()
		};
		let save = self.save;
		let file = self.file;
		let button = icon_text_button("folder", &preview, &theme)
			.width(Size::fill())
			.border_fill(theme.panel_border)
			.background(theme.bg)
			.hover_background(theme.panel_hover)
			.on_press(move |_| {
				let mut path = path.clone();
				spawn(async move {
					let new_path = tokio::task::spawn_blocking(move || {
						let dialog = FileDialog::new();
						if save {
							dialog.save_file()
						} else if file {
							dialog.pick_file()
						} else {
							dialog.pick_folder()
						}
					})
					.await
					.ok()
					.flatten();
					if let Some(new_path) = new_path {
						path.set(Some(new_path));
					}
				});
			});

		button
	}
}
