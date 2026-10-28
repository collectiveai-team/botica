import sys

from app.cli import main

with open("app.log", "a") as log:
    log.write("start\n")
sys.exit(main())
