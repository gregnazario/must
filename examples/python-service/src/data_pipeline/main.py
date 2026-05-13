import json
import sys

from data_pipeline import process


def main() -> None:
    raw = json.load(sys.stdin)
    result = process(raw)
    json.dump(result, sys.stdout, indent=2)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
