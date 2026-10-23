import polars as pl


def amounts(frame: pl.DataFrame) -> pl.DataFrame:
    return frame.with_columns(pl.col("amount").cast(pl.Int64))
