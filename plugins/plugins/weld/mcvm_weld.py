import sys
from smithed.weld import run_weld
import json
from pathlib import Path
import os

def output(method: str, data: object | None = None):
	if data is None:
		print(f"%_\"{method}\"")
	else:
		out = {}
		out[method] = data
		out2 = json.dumps(out)
		print(f"%_{out2}")

def weld_dir(dir: Path, ignore: list):
	# Dir to store unwelded files so that they persist across updates
	unwelded_path = dir.joinpath("unwelded")
	if not unwelded_path.exists():
		os.mkdir(unwelded_path)
	# Move all files to the unwelded dir
	for entry in os.listdir(dir):
		path = dir.joinpath(entry)

		if "weld_pack" in entry:
			continue
		for ignored in ignore:
			if ignored in entry:
				continue

		if path.is_file():
			target = unwelded_path.joinpath(entry)
			# It won't move if the target already exists
			if target.exists():
				os.remove(target)
			os.rename(path, target)

	# Now Weld
	beet_config = {
		"output": str(dir)
	}
	packs = [dir.joinpath(x) for x in os.listdir(unwelded_path)]
	with run_weld(packs=packs,config=beet_config,directory=dir) as ctx:
		ctx.data.save(path=dir.joinpath("weld_pack.zip"), overwrite=True)

def run():
	hook = sys.argv[1]
	if hook != "on_instance_setup":
		return
	
	arg_raw = sys.argv[2]

	arg = json.loads(arg_raw)

	if arg["update_depth"] == "shallow":
		return
	
	if "disable_weld" in arg["config"] and arg["config"]["disable_weld"]:
		return
	
	output("start_process")
	output("message", {
		"contents": {
			"StartProcess": "Welding packs"
		},
		"level": "important"
	})

	# Figure out the paths to load packs into
	game_dir = Path(arg["game_dir"])
	datapack_dirs = []
	datapack_folder = arg["config"]["datapack_folder"]
	if datapack_folder is not None:
		datapack_dirs = [game_dir.joinpath(datapack_folder)]
	else:
		if arg["side"] == "client":
			saves_dir = game_dir.joinpath("saves")
			# Trick to only get the immediate subdirectories
			for entry in next(os.walk(saves_dir))[1]:
				datapack_dirs.append(saves_dir.join(entry[0]))

		else:
			datapack_dirs = [game_dir.joinpath("world/datapacks")]

	resourcepack_dirs = [game_dir.joinpath("resourcepacks")]

	weld_ignore = []
	if "weld_ignore" in arg["config"]:
		weld_ignore = arg["config"]["weld_ignore"]

	# Run Weld on each directory
	for dir in datapack_dirs:
		weld_dir(dir, weld_ignore)

	for dir in resourcepack_dirs:
		weld_dir(dir, weld_ignore)

	output("message", {
		"contents": {
			"Success": "Packs welded",
		},
		"level": "important"
	})
	output("end_process")


def main():
	run()

output("set_result", {
	"main_class_override": None,
	"jar_path_override": None,
	"classpath_extension": []
})
