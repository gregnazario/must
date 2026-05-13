__version__ = "1.0.0"


def process(data: list[dict]) -> list[dict]:
    return [row for row in data if row.get("active", True)]
