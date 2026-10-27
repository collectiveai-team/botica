import warnings
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    import jax
else:
    import tensorflow as tf

with warnings.catch_warnings():
    import jax.numpy as jnp
