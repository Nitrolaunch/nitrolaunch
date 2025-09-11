# Shared World Files

On client instances, files placed in `.minecraft/world_files` will be shared across all of the worlds on that instance. They will also update live when new worlds are created.

For example, the file `.minecraft/world_files/directory/foo.txt` will be hardlinked to `.minecraft/saves/<world>/directory/foo.txt`