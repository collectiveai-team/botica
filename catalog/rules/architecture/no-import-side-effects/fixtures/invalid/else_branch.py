import shutil
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    pass
else:
    shutil.rmtree("build", ignore_errors=True)
