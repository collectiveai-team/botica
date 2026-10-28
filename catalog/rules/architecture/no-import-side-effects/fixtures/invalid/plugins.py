import os

PLUGINS = [name for name in os.listdir("plugins") if name.endswith(".py")]
