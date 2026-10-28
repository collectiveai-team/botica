import sqlite3


class Store:
    connection = sqlite3.connect("store.db")
