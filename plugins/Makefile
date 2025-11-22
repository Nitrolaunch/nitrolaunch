TARGET_DIR ?= "../target/release"

package: package.platform_dependent package.cross_platform

package.cross_platform: build.cross_platform
	@mkdir -p release

	@rm -f release/args-universal.zip
	@rm -f release/mojang_transfer-universal.zip
	@rm -f release/multiply-universal.zip
	@rm -f release/weld-universal.zip

	@zip -j release/args-universal.zip plugin.wasm=../target/wasm32-unknown-unknown/release/nitro_plugin_args.wasm plugins/args/plugin.json
	@zip -j release/mojang_transfer-universal.zip plugin.wasm=../target/wasm32-unknown-unknown/release/nitro_plugin_mojang_transfer.wasm plugins/mojang_transfer/plugin.json
	@zip -j release/multiply-universal.zip plugin.wasm=../target/wasm32-unknown-unknown/release/nitro_plugin_multiply.wasm plugins/multiply/plugin.json

	@zip -j release/gamepad-universal.zip plugins/gamepad/plugin.json plugins/gamepad/gamepad.js
	@zip -j release/weld-universal.zip plugins/weld/plugin.json plugins/weld/weld.pyz
	@cp plugins/extra_versions/plugin.json release/extra_versions-universal.json
	@cp plugins/lang/plugin.json release/lang-universal.json

build.cross_platform: build.weld
	@cargo build -p "nitro_plugin_*" --release --target wasm32-unknown-unknown

package.platform_dependent: build.platform_dependent package_zips.platform_dependent

build.platform_dependent:
	@cargo build --release --bins

package_zips.platform_dependent:
	@mkdir -p release
	
	@rm -f release/auto_mcs-${OS}.zip
	@rm -f release/automate-${OS}.zip
	@rm -f release/backup-${OS}.zip
	@rm -f release/beet-${OS}.zip
	@rm -f release/better_jsons-${OS}.zip
	@rm -f release/cleanup-${OS}.zip
	@rm -f release/config_split-${OS}.zip
	@rm -f release/curseforge_api-${OS}.zip
	@rm -f release/custom_files-${OS}.zip
	@rm -f release/docs-${OS}.zip
	@rm -f release/fabric_quilt-${OS}.zip
	@rm -f release/forge-${OS}.zip
	@rm -f release/gen_pkg-${OS}.zip
	@rm -f release/glfw_fix-${OS}.zip
	@rm -f release/modrinth-${OS}.zip
	@rm -f release/modrinth_api-${OS}.zip
	@rm -f release/multimc_transfer-${OS}.zip
	@rm -f release/nitro_transfer-${OS}.zip
	@rm -f release/options-${OS}.zip
	@rm -f release/packhost-${OS}.zip
	@rm -f release/paper-${OS}.zip
	@rm -f release/template_share-${OS}.zip
	@rm -f release/server_restart-${OS}.zip
	@rm -f release/smithed-${OS}.zip
	@rm -f release/smithed_api-${OS}.zip
	@rm -f release/sponge-${OS}.zip
	@rm -f release/stats-${OS}.zip
	@rm -f release/webtools-${OS}.zip
	@rm -f release/weld-${OS}.zip
	@rm -f release/xmcl_transfer-${OS}.zip

	@zip -j release/auto_mcs-${OS}.zip $(TARGET_DIR)/nitro_plugin_auto_mcs${EXTENSION} plugins/auto_mcs/plugin.json
	@zip -j release/automate-${OS}.zip $(TARGET_DIR)/nitro_plugin_automate${EXTENSION} plugins/automate/plugin.json
	@zip -j release/backup-${OS}.zip $(TARGET_DIR)/nitro_plugin_backup${EXTENSION} plugins/backup/plugin.json
	@zip -j release/beet-${OS}.zip $(TARGET_DIR)/nitro_plugin_beet${EXTENSION} plugins/beet/plugin.json
	@zip -j release/better_jsons-${OS}.zip $(TARGET_DIR)/nitro_plugin_better_jsons${EXTENSION} plugins/better_jsons/plugin.json
	@zip -j release/cleanup-${OS}.zip $(TARGET_DIR)/nitro_plugin_cleanup${EXTENSION} plugins/cleanup/plugin.json
	@zip -j release/config_split-${OS}.zip $(TARGET_DIR)/nitro_plugin_config_split${EXTENSION} plugins/config_split/plugin.json
	@zip -j release/curseforge_api-${OS}.zip $(TARGET_DIR)/nitro_plugin_curseforge_api${EXTENSION} plugins/curseforge_api/plugin.json
	@zip -j release/custom_files-${OS}.zip $(TARGET_DIR)/nitro_plugin_custom_files${EXTENSION} plugins/custom_files/plugin.json
	@zip -j release/docs-${OS}.zip $(TARGET_DIR)/nitro_plugin_docs${EXTENSION} plugins/docs/plugin.json
	@zip -j release/fabric_quilt-${OS}.zip $(TARGET_DIR)/nitro_plugin_fabric_quilt${EXTENSION} plugins/fabric_quilt/plugin.json
	@zip -j release/forge-${OS}.zip $(TARGET_DIR)/nitro_plugin_forge${EXTENSION} plugins/forge/plugin.json
	@zip -j release/gen_pkg-${OS}.zip $(TARGET_DIR)/nitro_plugin_gen_pkg${EXTENSION} plugins/gen_pkg/plugin.json
	@zip -j release/glfw_fix-${OS}.zip $(TARGET_DIR)/nitro_plugin_glfw_fix${EXTENSION} plugins/glfw_fix/plugin.json
	@zip -j release/modrinth-${OS}.zip $(TARGET_DIR)/nitro_plugin_modrinth${EXTENSION} plugins/modrinth/plugin.json
	@zip -j release/modrinth_api-${OS}.zip $(TARGET_DIR)/nitro_plugin_modrinth_api${EXTENSION} plugins/modrinth_api/plugin.json plugins/modrinth_api/page.html
	@zip -j release/multimc_transfer-${OS}.zip $(TARGET_DIR)/nitro_plugin_multimc_transfer${EXTENSION} plugins/multimc_transfer/plugin.json
	@zip -j release/nitro_transfer-${OS}.zip $(TARGET_DIR)/nitro_plugin_nitro_transfer${EXTENSION} plugins/nitro_transfer/plugin.json
	@zip -j release/options-${OS}.zip $(TARGET_DIR)/nitro_plugin_options${EXTENSION} plugins/options/plugin.json
	@zip -j release/packhost-${OS}.zip $(TARGET_DIR)/nitro_plugin_packhost${EXTENSION} plugins/packhost/plugin.json
	@zip -j release/paper-${OS}.zip $(TARGET_DIR)/nitro_plugin_paper${EXTENSION} plugins/paper/plugin.json
	@zip -j release/template_share-${OS}.zip $(TARGET_DIR)/nitro_plugin_template_share${EXTENSION} plugins/template_share/plugin.json plugins/template_share/import.js plugins/template_share/export.js
	@zip -j release/server_restart-${OS}.zip $(TARGET_DIR)/nitro_plugin_server_restart${EXTENSION} plugins/server_restart/plugin.json
	@zip -j release/smithed-${OS}.zip $(TARGET_DIR)/nitro_plugin_smithed${EXTENSION} plugins/smithed/plugin.json plugins/smithed/page.html
	@zip -j release/smithed_api-${OS}.zip $(TARGET_DIR)/nitro_plugin_smithed_api${EXTENSION} plugins/smithed_api/plugin.json
	@zip -j release/sponge-${OS}.zip $(TARGET_DIR)/nitro_plugin_sponge${EXTENSION} plugins/sponge/plugin.json
	@zip -j release/stats-${OS}.zip $(TARGET_DIR)/nitro_plugin_stats${EXTENSION} plugins/stats/plugin.json
	@zip -j release/webtools-${OS}.zip $(TARGET_DIR)/nitro_plugin_webtools${EXTENSION} plugins/webtools/plugin.json
	@zip -j release/xmcl_transfer-${OS}.zip $(TARGET_DIR)/nitro_plugin_xmcl_transfer${EXTENSION} plugins/xmcl_transfer/plugin.json

build.weld:
	@shiv -o plugins/weld/weld.pyz -e nitro_weld.main --site-packages plugins/weld smithed

install.args:
	@cargo build -p nitro_plugin_args --release --target wasm32-unknown-unknown
	@cp -r plugins/args ~/.local/share/nitro/plugins
	@cp -r ../target/wasm32-unknown-unknown/release/nitro_plugin_args.wasm ~/.local/share/nitro/plugins/args/plugin.wasm

install.auto_mcs:
	@cargo install --path . --bin nitro_plugin_auto_mcs --offline
	@mkdir -p ~/.local/share/nitro/plugins/auto_mcs
	@cat plugins/auto_mcs/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/auto_mcs/plugin.json

install.automate:
	@cargo install --path . --bin nitro_plugin_automate --offline
	@mkdir -p ~/.local/share/nitro/plugins/automate
	@cat plugins/automate/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/automate/plugin.json

install.backup:
	@cargo install --path . --bin nitro_plugin_backup --offline
	@mkdir -p ~/.local/share/nitro/plugins/backup
	@cat plugins/backup/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/backup/plugin.json

install.beet:
	@cargo install --path . --bin nitro_plugin_beet --offline
	@mkdir -p ~/.local/share/nitro/plugins/beet
	@cat plugins/beet/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/beet/plugin.json

install.better_jsons:
	@cargo install --path . --bin nitro_plugin_better_jsons --offline
	@mkdir -p ~/.local/share/nitro/plugins/better_jsons
	@cat plugins/better_jsons/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/better_jsons/plugin.json

install.cleanup:
	@cargo install --path . --bin nitro_plugin_cleanup --offline
	@mkdir -p ~/.local/share/nitro/plugins/cleanup
	@cat plugins/cleanup/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/cleanup/plugin.json

install.config_split:
	@cargo install --path . --bin nitro_plugin_config_split --offline
	@mkdir -p ~/.local/share/nitro/plugins/config_split
	@cat plugins/config_split/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/config_split/plugin.json

install.curseforge_api:
	@cargo install --path . --bin nitro_plugin_curseforge_api --offline
	@mkdir -p ~/.local/share/nitro/plugins/curseforge_api
	@cat plugins/curseforge_api/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/curseforge_api/plugin.json

install.custom_files:
	@cargo install --path . --bin nitro_plugin_custom_files --offline
	@mkdir -p ~/.local/share/nitro/plugins/custom_files
	@cat plugins/custom_files/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/custom_files/plugin.json

install.docs:
	@cargo install --path . --bin nitro_plugin_docs --offline
	@mkdir -p ~/.local/share/nitro/plugins/docs
	@cat plugins/docs/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/docs/plugin.json
	
install.extra_versions:
	@cp -r plugins/extra_versions ~/.local/share/nitro/plugins

install.fabric_quilt:
	@cargo install --path . --bin nitro_plugin_fabric_quilt --offline
	@mkdir -p ~/.local/share/nitro/plugins/fabric_quilt
	@cat plugins/fabric_quilt/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/fabric_quilt/plugin.json

install.forge:
	@cargo install --path . --bin nitro_plugin_forge --offline
	@mkdir -p ~/.local/share/nitro/plugins/forge
	@cat plugins/forge/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/forge/plugin.json

install.gamepad:
	@cp -r plugins/gamepad ~/.local/share/nitro/plugins

install.gen_pkg:
	@cargo install --path . --bin nitro_plugin_gen_pkg --offline
	@mkdir -p ~/.local/share/nitro/plugins/gen_pkg
	@cat plugins/gen_pkg/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/gen_pkg/plugin.json

install.glfw_fix:
	@cargo install --path . --bin nitro_plugin_glfw_fix --offline
	@mkdir -p ~/.local/share/nitro/plugins/glfw_fix
	@cat plugins/glfw_fix/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/glfw_fix/plugin.json

install.lang:
	@cp -r plugins/lang ~/.local/share/nitro/plugins

install.modrinth:
	@cargo install --path . --bin nitro_plugin_modrinth --offline
	@cp -r plugins/modrinth ~/.local/share/nitro/plugins
	@cat plugins/modrinth/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/modrinth/plugin.json

install.modrinth_api:
	@cargo install --path . --bin nitro_plugin_modrinth_api --offline
	@mkdir -p ~/.local/share/nitro/plugins/modrinth_api
	@cat plugins/modrinth_api/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/modrinth_api/plugin.json

install.mojang_transfer:
	@cargo build -p nitro_plugin_mojang_transfer --release --target wasm32-unknown-unknown
	@cp -r plugins/mojang_transfer ~/.local/share/nitro/plugins
	@cp -r ../target/wasm32-unknown-unknown/release/nitro_plugin_mojang_transfer.wasm ~/.local/share/nitro/plugins/mojang_transfer/plugin.wasm

install.multimc_transfer:
	@cargo install --path . --bin nitro_plugin_multimc_transfer --offline
	@mkdir -p ~/.local/share/nitro/plugins/multimc_transfer
	@cat plugins/multimc_transfer/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/multimc_transfer/plugin.json

install.multiply:
	@cargo build -p nitro_plugin_multiply --release --target wasm32-unknown-unknown
	@cp -r plugins/multiply ~/.local/share/nitro/plugins
	@cp -r ../target/wasm32-unknown-unknown/release/nitro_plugin_multiply.wasm ~/.local/share/nitro/plugins/multiply/plugin.wasm

install.nitro_transfer:
	@cargo install --path . --bin nitro_plugin_nitro_transfer --offline
	@mkdir -p ~/.local/share/nitro/plugins/nitro_transfer
	@cat plugins/nitro_transfer/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/nitro_transfer/plugin.json

install.options:
	@cargo install --path . --bin nitro_plugin_options --offline
	@mkdir -p ~/.local/share/nitro/plugins/options
	@cat plugins/options/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/options/plugin.json

install.packhost:
	@cargo install --path . --bin nitro_plugin_packhost --offline
	@mkdir -p ~/.local/share/nitro/plugins/packhost
	@cat plugins/packhost/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/packhost/plugin.json

install.paper:
	@cargo install --path . --bin nitro_plugin_paper --offline
	@mkdir -p ~/.local/share/nitro/plugins/paper
	@cat plugins/paper/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/paper/plugin.json

install.template_share:
	@cargo install --path . --bin nitro_plugin_template_share --offline
	@mkdir -p ~/.local/share/nitro/plugins/template_share
	@cat plugins/template_share/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/template_share/plugin.json
	@cp plugins/template_share/import.js ~/.local/share/nitro/plugins/template_share
	@cp plugins/template_share/export.js ~/.local/share/nitro/plugins/template_share

install.server_restart:
	@cargo install --path . --bin nitro_plugin_server_restart --offline
	@mkdir -p ~/.local/share/nitro/plugins/server_restart
	@cat plugins/server_restart/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/server_restart/plugin.json

install.smithed:
	@cargo install --path . --bin nitro_plugin_smithed --offline
	@cp -r plugins/smithed ~/.local/share/nitro/plugins
	@cat plugins/smithed/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/smithed/plugin.json

install.smithed_api:
	@cargo install --path . --bin nitro_plugin_smithed_api --offline
	@mkdir -p ~/.local/share/nitro/plugins/smithed_api
	@cat plugins/smithed_api/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/smithed_api/plugin.json

install.sponge:
	@cargo install --path . --bin nitro_plugin_sponge --offline
	@mkdir -p ~/.local/share/nitro/plugins/sponge
	@cat plugins/sponge/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/sponge/plugin.json

install.stats:
	@cargo install --path . --bin nitro_plugin_stats --offline
	@mkdir -p ~/.local/share/nitro/plugins/stats
	@cat plugins/stats/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/stats/plugin.json

install.webtools:
	@cargo install --path . --bin nitro_plugin_webtools --offline
	@cp -r plugins/webtools ~/.local/share/nitro/plugins
	@cat plugins/webtools/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/webtools/plugin.json

install.weld:
	@mkdir -p ~/.local/share/nitro/plugins/weld
	@cp plugins/weld/nitro_weld.py ~/.local/share/nitro/plugins/weld
	@cp plugins/weld/local_plugin.json ~/.local/share/nitro/plugins/weld/plugin.json

install.xmcl_transfer:
	@cargo install --path . --bin nitro_plugin_xmcl_transfer --offline
	@mkdir -p ~/.local/share/nitro/plugins/xmcl_transfer
	@cat plugins/xmcl_transfer/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/xmcl_transfer/plugin.json
