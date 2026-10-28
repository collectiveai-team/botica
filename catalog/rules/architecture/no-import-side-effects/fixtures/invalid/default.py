import pandas as pd


def score(rows: list[int], table=pd.read_csv("weights.csv")) -> float:
    return float(table.loc[rows].sum())
