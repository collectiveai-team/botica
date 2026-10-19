import sys

import cyclopts

app = cyclopts.App(name=sys.argv[0])

if __name__ == "__main__":
    app(sys.argv[1:])
    app(sys.argv)
