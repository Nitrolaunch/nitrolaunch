let html = `
	<div id="profile-import-prompt" class="cont col" style="padding: 2rem">
		<div class="cont bold">Import Profile</div>
		<div class="cont start label">
			<label for="code">CODE</label>
		</div>
		<input
			type="text"
			id="profile-import-code"
			name="code"
		></input>
		<div class="cont start label">
			<label for="id">ID FOR NEW PROFILE</label>
		</div>
		<input
			type="text"
			id="profile-import-id"
			name="id"
		></input>
		<div class="cont">
			<button
				style="border: var(--border) solid var(--bg3)"
				id="profile-import-cancel"
			>
				Cancel
			</button>
			<button
				style="border: var(--border) solid var(--bg3)"
				id="profile-import-submit"
			>
				Import
			</button>
		</div>
	</div>
`;

setModal(html);

globalInterval("load_profile_import", () => {
	let prompt = document.getElementById("profile-import-prompt");
	if (prompt != undefined && prompt.dataset.loaded == undefined) {
		document.getElementById("profile-import-id").addEventListener("change", (e) => {
			e.target.value = sanitizeInstanceId(e.target.value);
		});
		document.getElementById("profile-import-id").addEventListener("keyup", (e) => {
			e.target.value = sanitizeInstanceId(e.target.value);
		});
		document.getElementById("profile-import-cancel").addEventListener("click", () => {
			setModal(undefined);
		});
		document.getElementById("profile-import-submit").addEventListener("click", async () => {
			let code = document.getElementById("profile-import-code").value;
			let id = document.getElementById("profile-import-id").value;

			startTask("Importing Profile");
			try {
				await customAction("profile_share", "import_profile", { id: id, code: code });

				showSuccessToast("Profile imported");
				updateInstanceList();
			} catch (e) {
				showErrorToast("Failed to import profile: " + e);
			}
			endTask("Importing Profile");

			setModal(undefined);
		});
		prompt.dataset.loaded = "true";
	}
}, 100);
