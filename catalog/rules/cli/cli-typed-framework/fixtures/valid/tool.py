import cyclopts

app = cyclopts.App()


@app.command
def greet(name: str, *, loud: bool = False) -> None: ...
