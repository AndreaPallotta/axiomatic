#!/usr/bin/env python3
"""
Axiomatic - Hugging Face Hub Model Sync Utility
Allows publishing and retrieving trained MCTS neural theorem proving checkpoints.
"""

import argparse
import sys
from pathlib import Path

try:
    from huggingface_hub import HfApi, hf_hub_download
except ImportError:
    print("[ERROR] huggingface_hub package is required. Install via: pip install huggingface_hub")
    sys.exit(1)

def upload_model(repo_id: str, checkpoint_path: str = "models/checkpoint_latest.json"):
    api = HfApi()
    print(f"[INFO] Uploading {checkpoint_path} to Hugging Face repository {repo_id}...")
    api.upload_file(
        path_or_fileobj=checkpoint_path,
        path_in_repo="checkpoint_latest.json",
        repo_id=repo_id,
        repo_type="model"
    )
    print(f"[SUCCESS] Checkpoint published to https://huggingface.co/{repo_id}")

def download_model(repo_id: str, dest_dir: str = "models"):
    print(f"[INFO] Downloading latest checkpoint from https://huggingface.co/{repo_id}...")
    dest = Path(dest_dir)
    dest.mkdir(parents=True, exist_ok=True)
    file_path = hf_hub_download(
        repo_id=repo_id,
        filename="checkpoint_latest.json",
        local_dir=str(dest)
    )
    print(f"[SUCCESS] Checkpoint downloaded to {file_path}")

def main():
    parser = argparse.ArgumentParser(description="Sync Axiomatic checkpoints with Hugging Face Hub")
    subparsers = parser.add_subparsers(dest="action", required=True)

    up_parser = subparsers.add_parser("upload", help="Upload checkpoint to HF Hub")
    up_parser.add_argument("--repo", required=True, help="HF repository ID (e.g. AndreaPallotta/axiomatic-policy)")
    up_parser.add_argument("--file", default="models/checkpoint_latest.json", help="Path to checkpoint JSON")

    down_parser = subparsers.add_parser("download", help="Download checkpoint from HF Hub")
    down_parser.add_argument("--repo", required=True, help="HF repository ID (e.g. AndreaPallotta/axiomatic-policy)")
    down_parser.add_argument("--dest", default="models", help="Destination folder")

    args = parser.parse_args()

    if args.action == "upload":
        upload_model(args.repo, args.file)
    elif args.action == "download":
        download_model(args.repo, args.dest)

if __name__ == "__main__":
    main()
