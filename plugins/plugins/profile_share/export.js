globalInterval("profile_share_export", () => {
	let footer = document.getElementById("footer");
	if (footer.dataset.mode == "profile") {
		let button = document.getElementById("profile-share-export");
		if (button == undefined) {
			let elem = document.createElement("button");
			elem.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="1rem" height="1rem" viewBox="0 0 16 16" fill="currentColor"><path d="m6 2c-2.216 0-4 1.784-4 4v4c0 2.216 1.784 4 4 4h4c2.216 0 4-1.784 4-4h-2c0 1.108-0.892 2-2 2h-4c-1.108 0-2-0.892-2-2v-4c0-1.108 0.892-2 2-2v-2zm2 0v2h2c0.17886 0 0.35087 0.02667 0.51562 0.070312l-3.2227 3.2227a1 1 0 0 0 0 1.4141 1 1 0 0 0 1.4141 0l3.2227-3.2227c0.043642 0.16476 0.070312 0.33677 0.070312 0.51562v2h2v-2c0-2.216-1.784-4-4-4h-2z" /></svg>`;

			elem.id = "profile-share-export";
			elem.className = "cont";
			elem.style.width = "2rem";
			elem.style.height = "2rem";
			elem.style.padding = "0";
			elem.style.aspectRatio = "1";
			elem.title = "Export this profile as a code";

			elem.addEventListener("click", async () => {
				let profileId = footer.dataset.selectedItem;
				if (profileId == undefined) {
					showErrorToast("Profile ID not set in footer");
				}
				
				startTask("Exporting Profile");
				try {
					let code = await customAction("profile_share", "export_profile", profileId);
					
					await copyToClipboard(code);
					showSuccessToast("Code copied to clipboard");
				} catch (e) {
					showErrorToast("Failed to export profile: " + e);
				}
				endTask("Exporting Profile");
			})

			document.getElementById("footer-left-buttons").appendChild(elem);
			button = elem;
		}
		button.style.display = "";
	} else {
		let button = document.getElementById("profile-share-export");
		if (button != undefined) {
			button.style.display = "none";
		}
	}
}, 50);