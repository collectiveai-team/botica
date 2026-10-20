# Release

The release workflow runs on a `v*` tag. It builds the wheels, publishes them to PyPI and
copies the hooks to the mirror. A step that fails stops the workflow.
