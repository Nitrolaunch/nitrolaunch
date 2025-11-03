let html = `
	<div id="template-import-prompt" class="cont col" style="padding: 2rem">
		<div class="cont bold">Import Template</div>
		<div class="cont start label">
			<label for="code">CODE</label>
		</div>
		<input
			type="text"
			id="template-import-code"
			name="code"
		></input>
		<div class="cont start label">
			<label for="id">ID FOR NEW TEMPLATE</label>
		</div>
		<input
			type="text"
			id="template-import-id"
			name="id"
		></input>
		<div class="cont">
			<button
				style="border: var(--border) solid var(--bg3)"
				id="template-import-cancel"
			>
				Cancel
			</button>
			<button
				style="border: var(--border) solid var(--bg3)"
				id="template-import-submit"
			>
				Import
			</button>
		</div>
	</div>
`;

setModal(html);

globalInterval("load_template_import", () => {
	let prompt = document.getElementById("template-import-prompt");
	if (prompt != undefined && prompt.dataset.loaded == undefined) {
		document.getElementById("template-import-id").addEventListener("change", (e) => {
			e.target.value = sanitizeInstanceId(e.target.value);
		});
		document.getElementById("template-import-id").addEventListener("keyup", (e) => {
			e.target.value = sanitizeInstanceId(e.target.value);
		});
		document.getElementById("template-import-cancel").addEventListener("click", () => {
			setModal(undefined);
		});
		document.getElementById("template-import-submit").addEventListener("click", async () => {
			let code = document.getElementById("template-import-code").value;
			let id = document.getElementById("template-import-id").value;

			startTask("Importing Template");
			try {
				await customAction("template_share", "import_template", { id: id, code: code });

				showSuccessToast("Template imported");
				updateInstanceList();
			} catch (e) {
				showErrorToast("Failed to import template: " + e);
			}
			endTask("Importing Template");

			setModal(undefined);
		});
		prompt.dataset.loaded = "true";
	}
}, 100);
