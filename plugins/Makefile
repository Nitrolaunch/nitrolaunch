package: package.platform_dependent package.cross_platform

package.cross_platform: build.weld
	@mkdir -p release
	@rm -f release/weld-universal.zip && zip -j release/weld-universal.zip plugins/weld/plugin.json plugins/weld/weld.pyz
	@cp plugins/extra_versions/plugin.json release/extra_versions-universal.json
	@cp plugins/lang/plugin.json release/lang-universal.json

package.platform_dependent: build.platform_dependent package_zips.platform_dependent

build.platform_dependent:
	@cargo build --release --bins

TARGET_DIR ?= "../target/release"

package_zips.platform_dependent:
	@mkdir -p release
	
	@rm -f release/archive-${OS}.zip
	@rm -f release/args-${OS}.zip
	@rm -f release/automate-${OS}.zip
	@rm -f release/backup-${OS}.zip
	@rm -f release/better_jsons-${OS}.zip
	@rm -f release/config_split-${OS}.zip
	@rm -f release/custom_files-${OS}.zip
	@rm -f release/docs-${OS}.zip
	@rm -f release/fabric_quilt-${OS}.zip
	@rm -f release/forge-${OS}.zip
	@rm -f release/gen_pkg-${OS}.zip
	@rm -f release/glfw_fix-${OS}.zip
	@rm -f release/nitro_transfer-${OS}.zip
	@rm -f release/modrinth-${OS}.zip
	@rm -f release/modrinth_api-${OS}.zip
	@rm -f release/multimc_transfer-${OS}.zip
	@rm -f release/options-${OS}.zip
	@rm -f release/packhost-${OS}.zip
	@rm -f release/paper-${OS}.zip
	@rm -f release/server_restart-${OS}.zip
	@rm -f release/smithed-${OS}.zip
	@rm -f release/smithed_api-${OS}.zip
	@rm -f release/sponge-${OS}.zip
	@rm -f release/stats-${OS}.zip
	@rm -f release/webtools-${OS}.zip
	@rm -f release/weld-${OS}.zip
	@rm -f release/xmcl_transfer-${OS}.zip

	@zip -j release/archive-${OS}.zip $(TARGET_DIR)/nitro_plugin_archive${EXTENSION} plugins/archive/plugin.json
	@zip -j release/args-${OS}.zip $(TARGET_DIR)/nitro_plugin_args${EXTENSION} plugins/args/plugin.json
	@zip -j release/automate-${OS}.zip $(TARGET_DIR)/nitro_plugin_automate${EXTENSION} plugins/automate/plugin.json
	@zip -j release/backup-${OS}.zip $(TARGET_DIR)/nitro_plugin_backup${EXTENSION} plugins/backup/plugin.json
	@zip -j release/better_jsons-${OS}.zip $(TARGET_DIR)/nitro_plugin_better_jsons${EXTENSION} plugins/better_jsons/plugin.json
	@zip -j release/config_split-${OS}.zip $(TARGET_DIR)/nitro_plugin_config_split${EXTENSION} plugins/config_split/plugin.json
	@zip -j release/custom_files-${OS}.zip $(TARGET_DIR)/nitro_plugin_custom_files${EXTENSION} plugins/custom_files/plugin.json
	@zip -j release/docs-${OS}.zip $(TARGET_DIR)/nitro_plugin_docs${EXTENSION} plugins/docs/plugin.json
	@zip -j release/fabric_quilt-${OS}.zip $(TARGET_DIR)/nitro_plugin_fabric_quilt${EXTENSION} plugins/fabric_quilt/plugin.json
	@zip -j release/forge-${OS}.zip $(TARGET_DIR)/nitro_plugin_forge${EXTENSION} plugins/forge/plugin.json
	@zip -j release/gen_pkg-${OS}.zip $(TARGET_DIR)/nitro_plugin_gen_pkg${EXTENSION} plugins/gen_pkg/plugin.json
	@zip -j release/glfw_fix-${OS}.zip $(TARGET_DIR)/nitro_plugin_glfw_fix${EXTENSION} plugins/glfw_fix/plugin.json
	@zip -j release/nitro_transfer-${OS}.zip $(TARGET_DIR)/nitro_plugin_nitro_transfer${EXTENSION} plugins/nitro_transfer/plugin.json
	@zip -j release/modrinth-${OS}.zip $(TARGET_DIR)/nitro_plugin_modrinth${EXTENSION} plugins/modrinth/plugin.json
	@zip -j release/modrinth_api-${OS}.zip $(TARGET_DIR)/nitro_plugin_modrinth_api${EXTENSION} plugins/modrinth_api/plugin.json plugins/modrinth_api/page.html
	@zip -j release/multimc_transfer-${OS}.zip $(TARGET_DIR)/nitro_plugin_multimc_transfer${EXTENSION} plugins/multimc_transfer/plugin.json
	@zip -j release/options-${OS}.zip $(TARGET_DIR)/nitro_plugin_options${EXTENSION} plugins/options/plugin.json
	@zip -j release/packhost-${OS}.zip $(TARGET_DIR)/nitro_plugin_packhost${EXTENSION} plugins/packhost/plugin.json
	@zip -j release/paper-${OS}.zip $(TARGET_DIR)/nitro_plugin_paper${EXTENSION} plugins/paper/plugin.json
	@zip -j release/server_restart-${OS}.zip $(TARGET_DIR)/nitro_plugin_server_restart${EXTENSION} plugins/server_restart/plugin.json
	@zip -j release/smithed-${OS}.zip $(TARGET_DIR)/nitro_plugin_smithed${EXTENSION} plugins/smithed/plugin.json plugins/smithed/page.html
	@zip -j release/smithed_api-${OS}.zip $(TARGET_DIR)/nitro_plugin_smithed_api${EXTENSION} plugins/smithed_api/plugin.json
	@zip -j release/sponge-${OS}.zip $(TARGET_DIR)/nitro_plugin_sponge${EXTENSION} plugins/sponge/plugin.json
	@zip -j release/stats-${OS}.zip $(TARGET_DIR)/nitro_plugin_stats${EXTENSION} plugins/stats/plugin.json
	@zip -j release/webtools-${OS}.zip $(TARGET_DIR)/nitro_plugin_webtools${EXTENSION} plugins/webtools/plugin.json
	@zip -j release/xmcl_transfer-${OS}.zip $(TARGET_DIR)/nitro_plugin_xmcl_transfer${EXTENSION} plugins/xmcl_transfer/plugin.json

build.weld:
	@shiv -o plugins/weld/weld.pyz -e nitro_weld.main --site-packages plugins/weld smithed

install.archive:
	@cargo install --path . --bin nitro_plugin_archive --offline
	@cat plugins/archive/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/archive.json

install.args:
	@cargo install --path . --bin nitro_plugin_args --offline
	@cat plugins/args/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/args.json
	
install.automate:
	@cargo install --path . --bin nitro_plugin_automate --offline
	@cat plugins/automate/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/automate.json

install.backup:
	@cargo install --path . --bin nitro_plugin_backup --offline
	@cat plugins/backup/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/backup.json

install.better_jsons:
	@cargo install --path . --bin nitro_plugin_better_jsons --offline
	@cat plugins/better_jsons/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/better_jsons.json

install.config_split:
	@cargo install --path . --bin nitro_plugin_config_split --offline
	@cat plugins/config_split/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/config_split.json

install.custom_files:
	@cargo install --path . --bin nitro_plugin_custom_files --offline
	@cat plugins/custom_files/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/custom_files.json

install.docs:
	@cargo install --path . --bin nitro_plugin_docs --offline
	@cat plugins/docs/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/docs.json
	
install.extra_versions:
	@cp plugins/extra_versions/plugin.json ~/.local/share/nitro/plugins/extra_versions.json

install.fabric_quilt:
	@cargo install --path . --bin nitro_plugin_fabric_quilt --offline
	@cat plugins/fabric_quilt/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/fabric_quilt.json

install.forge:
	@cargo install --path . --bin nitro_plugin_forge --offline
	@cat plugins/forge/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/forge.json

install.gen_pkg:
	@cargo install --path . --bin nitro_plugin_gen_pkg --offline
	@cat plugins/gen_pkg/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/gen_pkg.json

install.glfw_fix:
	@cargo install --path . --bin nitro_plugin_glfw_fix --offline
	@cat plugins/glfw_fix/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/glfw_fix.json

install.lang:
	@cp plugins/lang/plugin.json ~/.local/share/nitro/plugins/lang.json

install.nitro_transfer:
	@cargo install --path . --bin nitro_plugin_nitro_transfer --offline
	@cat plugins/nitro_transfer/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/nitro_transfer.json

install.modrinth:
	@cargo install --path . --bin nitro_plugin_modrinth --offline
	@cp -r plugins/modrinth ~/.local/share/nitro/plugins
	@cat plugins/modrinth/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/modrinth/plugin.json

install.modrinth_api:
	@cargo install --path . --bin nitro_plugin_modrinth_api --offline
	@cat plugins/modrinth_api/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/modrinth_api.json

install.multimc_transfer:
	@cargo install --path . --bin nitro_plugin_multimc_transfer --offline
	@cat plugins/multimc_transfer/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/multimc_transfer.json

install.options:
	@cargo install --path . --bin nitro_plugin_options --offline
	@cat plugins/options/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/options.json

install.packhost:
	@cargo install --path . --bin nitro_plugin_packhost --offline
	@cat plugins/packhost/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/packhost.json

install.paper:
	@cargo install --path . --bin nitro_plugin_paper --offline
	@cat plugins/paper/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/paper.json

install.server_restart:
	@cargo install --path . --bin nitro_plugin_server_restart --offline
	@cat plugins/server_restart/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/server_restart.json

install.smithed:
	@cargo install --path . --bin nitro_plugin_smithed --offline
	@cp -r plugins/smithed ~/.local/share/nitro/plugins
	@cat plugins/smithed/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/smithed/plugin.json

install.smithed_api:
	@cargo install --path . --bin nitro_plugin_smithed_api --offline
	@cat plugins/smithed_api/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/smithed_api.json

install.sponge:
	@cargo install --path . --bin nitro_plugin_sponge --offline
	@cat plugins/sponge/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/sponge.json

install.stats:
	@cargo install --path . --bin nitro_plugin_stats --offline
	@cat plugins/stats/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/stats.json

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
	@cat plugins/xmcl_transfer/plugin.json | sed 's,\$${PLUGIN_DIR}/,,' > ~/.local/share/nitro/plugins/xmcl_transfer.json
