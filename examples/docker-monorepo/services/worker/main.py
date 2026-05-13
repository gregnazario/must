import time
import os


def main():
    interval = int(os.environ.get("POLL_INTERVAL", "30"))
    print(f"Worker started, polling every {interval}s")
    while True:
        process_jobs()
        time.sleep(interval)


def process_jobs():
    print("Checking for jobs...")


if __name__ == "__main__":
    main()
