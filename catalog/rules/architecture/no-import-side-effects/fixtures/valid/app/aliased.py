import subprocess as sp
from os import path

RUNNER = sp.run
EXISTS = path.isfile("setup.cfg")
