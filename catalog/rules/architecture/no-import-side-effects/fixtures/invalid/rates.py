import requests

try:
    RATES = requests.get("https://example.com/rates", timeout=5).json()
except requests.RequestException:
    RATES = {}
