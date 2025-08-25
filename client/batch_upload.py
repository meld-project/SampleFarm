#!/usr/bin/env python3
"""
SampleFarm Batch File Upload Tool

Copyright (c) 2024 SampleFarm Project

This work is licensed under CC BY-NC-SA 4.0
https://creativecommons.org/licenses/by-nc-sa/4.0/

Usage:
    python batch_upload.py <directory_path> [options]

Examples:
    # Upload all files in directory, mark as malicious samples
    python batch_upload.py /path/to/samples --type malicious --source "honeypot"
    
    # Upload all files with labels
    python batch_upload.py /path/to/samples --type malicious --labels trojan,backdoor
    
    # Recursively upload files in subdirectories
    python batch_upload.py /path/to/samples --recursive --type benign
    
    # Upload only specific file extensions
    python batch_upload.py /path/to/samples --extensions exe,dll,zip --type malicious
    
    # Concurrent upload for better performance
    python batch_upload.py /path/to/samples --workers 5 --type malicious
"""

import os
import sys
import json
import time
import argparse
import hashlib
import mimetypes
from pathlib import Path
from typing import List, Dict, Optional, Any
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime

import requests
from requests_toolbelt import MultipartEncoder
from tqdm import tqdm
from colorama import init, Fore, Style

# Initialize colorama for Windows terminal color support
init(autoreset=True)

# Default configuration
DEFAULT_API_BASE_URL = "http://localhost:8080"
DEFAULT_WORKERS = 3
DEFAULT_TIMEOUT = 300  # 5-minute timeout
MAX_FILE_SIZE = 1 * 1024 * 1024 * 1024  # 1GB

class SampleFarmUploader:
    """SampleFarm batch upload client"""
    
    def __init__(self, base_url: str = DEFAULT_API_BASE_URL):
        self.base_url = base_url.rstrip('/')
        self.session = requests.Session()
        self.upload_url = f"{self.base_url}/api/samples/upload"
        
        # Test connection
        self._test_connection()
    
    def _test_connection(self):
        """Test API server connection"""
        try:
            response = self.session.get(f"{self.base_url}/api/status", timeout=5)
            if response.status_code == 200:
                data = response.json()
                if data.get('code') == 200:
                    print(f"{Fore.GREEN}✓ Successfully connected to SampleFarm server: {self.base_url}")
                    return
            raise Exception(f"Server returned error status: {response.status_code}")
        except Exception as e:
            print(f"{Fore.RED}✗ Cannot connect to SampleFarm server: {e}")
            sys.exit(1)
    
    def upload_file(self, file_path: Path, metadata: Dict[str, Any]) -> Dict[str, Any]:
        """Upload a single file"""
        # Check file size
        file_size = file_path.stat().st_size
        if file_size > MAX_FILE_SIZE:
            raise ValueError(f"File too large: {file_size / (1024*1024):.2f} MB, maximum supported: {MAX_FILE_SIZE / (1024*1024):.2f} MB")
        
        # Prepare multipart data
        with open(file_path, 'rb') as f:
            # Use MultipartEncoder to properly handle multipart/form-data
            encoder = MultipartEncoder(
                fields={
                    'file': (file_path.name, f, self._get_mime_type(file_path)),
                    'metadata': json.dumps(metadata)
                }
            )
            
            # Send request with correct Content-Type
            response = self.session.post(
                self.upload_url,
                data=encoder,
                headers={'Content-Type': encoder.content_type},
                timeout=DEFAULT_TIMEOUT
            )
        
        # Handle response
        if response.status_code == 200:
            result = response.json()
            if result.get('code') == 200:
                return result['data']
            else:
                raise Exception(f"API error: {result.get('msg', 'Unknown error')}")
        else:
            raise Exception(f"HTTP error {response.status_code}: {response.text}")
    
    def _get_mime_type(self, file_path: Path) -> str:
        """Get file MIME type"""
        mime_type, _ = mimetypes.guess_type(str(file_path))
        return mime_type or 'application/octet-stream'


def calculate_file_hash(file_path: Path, algorithm: str = 'sha256') -> str:
    """Calculate file hash"""
    hasher = hashlib.new(algorithm)
    with open(file_path, 'rb') as f:
        while chunk := f.read(8192):
            hasher.update(chunk)
    return hasher.hexdigest()


def scan_directory(directory: Path, recursive: bool = False, 
                  extensions: Optional[List[str]] = None) -> List[Path]:
    """Scan directory to get file list"""
    files = []
    
    # Determine scan pattern
    if recursive:
        pattern = '**/*'
    else:
        pattern = '*'
    
    # Scan files
    for path in directory.glob(pattern):
        if path.is_file():
            # Check extension filter
            if extensions:
                if not any(path.suffix.lower() == f'.{ext.lower()}' for ext in extensions):
                    continue
            
            # Skip hidden files
            if path.name.startswith('.'):
                continue
            
            files.append(path)
    
    return sorted(files)


def upload_worker(uploader: SampleFarmUploader, file_path: Path, 
                 metadata: Dict[str, Any], pbar: tqdm) -> Dict[str, Any]:
    """Upload worker thread"""
    try:
        # Calculate file hash (for local deduplication check)
        file_hash = calculate_file_hash(file_path)
        
        # Upload file
        result = uploader.upload_file(file_path, metadata)
        
        # Update progress bar
        pbar.update(1)
        
        return {
            'success': True,
            'file': str(file_path),
            'size': file_path.stat().st_size,
            'sample_id': result.get('sample_id'),
            'is_duplicate': result.get('is_duplicate', False),
            'hash': file_hash
        }
    except Exception as e:
        pbar.update(1)
        return {
            'success': False,
            'file': str(file_path),
            'error': str(e)
        }


def main():
    parser = argparse.ArgumentParser(
        description='SampleFarm batch file upload tool',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__
    )
    
    # Required arguments
    parser.add_argument('directory', type=str, help='Directory path containing files to upload')
    
    # Sample type (required)
    parser.add_argument('--type', '-t', required=True, 
                       choices=['benign', 'malicious'],
                       help='Sample type: benign (safe) or malicious (harmful)')
    
    # Optional arguments
    parser.add_argument('--source', '-s', type=str, default='batch_upload',
                       help='Sample source (default: batch_upload)')
    
    parser.add_argument('--labels', '-l', type=str,
                       help='Label list separated by comma (e.g.: trojan,backdoor)')
    
    parser.add_argument('--recursive', '-r', action='store_true',
                       help='Recursively scan subdirectories')
    
    parser.add_argument('--extensions', '-e', type=str,
                       help='Upload only specified file extensions, comma-separated (e.g.: exe,dll,zip)')
    
    parser.add_argument('--workers', '-w', type=int, default=DEFAULT_WORKERS,
                       help=f'Number of concurrent upload threads (default: {DEFAULT_WORKERS})')
    
    parser.add_argument('--api-url', '-u', type=str, default=DEFAULT_API_BASE_URL,
                       help=f'API server URL (default: {DEFAULT_API_BASE_URL})')
    
    parser.add_argument('--passwords', '-p', type=str,
                       help='ZIP file password list, comma-separated (e.g.: infected,malware)')
    
    parser.add_argument('--custom-metadata', '-m', type=str,
                       help='Custom metadata in JSON format string')
    
    parser.add_argument('--dry-run', action='store_true',
                       help='Only scan files without actual upload')
    
    parser.add_argument('--skip-duplicates', action='store_true',
                       help='Skip duplicate files (based on local hash check)')
    
    args = parser.parse_args()
    
    # Validate directory
    directory = Path(args.directory)
    if not directory.exists():
        print(f"{Fore.RED}✗ Directory does not exist: {directory}")
        sys.exit(1)
    
    if not directory.is_dir():
        print(f"{Fore.RED}✗ Not a valid directory: {directory}")
        sys.exit(1)
    
    # Parse extension list
    extensions = None
    if args.extensions:
        extensions = [ext.strip().lower() for ext in args.extensions.split(',')]
    
    # Parse label list
    labels = None
    if args.labels:
        labels = [label.strip() for label in args.labels.split(',')]
    
    # Parse password list
    passwords = None
    if args.passwords:
        passwords = [pwd.strip() for pwd in args.passwords.split(',')]
    
    # Parse custom metadata
    custom_metadata = None
    if args.custom_metadata:
        try:
            custom_metadata = json.loads(args.custom_metadata)
        except json.JSONDecodeError as e:
            print(f"{Fore.RED}✗ Invalid JSON format metadata: {e}")
            sys.exit(1)
    
    # Scan files
    print(f"\n{Fore.CYAN}Scanning directory: {directory}")
    files = scan_directory(directory, args.recursive, extensions)
    
    if not files:
        print(f"{Fore.YELLOW}⚠ No matching files found")
        return
    
    # Show scan results
    total_size = sum(f.stat().st_size for f in files)
    print(f"{Fore.GREEN}✓ Found {len(files)} files, total size: {total_size / (1024*1024):.2f} MB")
    
    if args.dry_run:
        print(f"\n{Fore.YELLOW}Dry run mode, file list:")
        for f in files[:10]:
            print(f"  - {f.name} ({f.stat().st_size / 1024:.1f} KB)")
        if len(files) > 10:
            print(f"  ... and {len(files) - 10} more files")
        return
    
    # Confirm upload
    print(f"\n{Fore.YELLOW}Upload configuration:")
    print(f"  - Sample type: {args.type}")
    print(f"  - Sample source: {args.source}")
    if labels:
        print(f"  - Labels: {', '.join(labels)}")
    if passwords:
        print(f"  - ZIP passwords: {len(passwords)} items")
    print(f"  - Concurrent threads: {args.workers}")
    
    response = input(f"\n{Fore.CYAN}Confirm to start upload? [y/N]: ")
    if response.lower() != 'y':
        print("Cancelled")
        return
    
    # Create upload client
    uploader = SampleFarmUploader(args.api_url)
    
    # Prepare metadata
    metadata = {
        'sample_type': args.type.capitalize(),  # API requires capitalized first letter
        'source': args.source,
    }
    
    if labels:
        metadata['labels'] = labels
    
    if passwords:
        metadata['passwords'] = passwords
    
    if custom_metadata:
        metadata['custom_metadata'] = custom_metadata
    
    # Start upload
    print(f"\n{Fore.CYAN}Starting upload...")
    start_time = time.time()
    
    results = {
        'success': 0,
        'failed': 0,
        'duplicate': 0,
        'errors': []
    }
    
    # Processed file hashes (for local deduplication)
    processed_hashes = set()
    
    # Create progress bar
    with tqdm(total=len(files), desc="Upload progress", unit="files") as pbar:
        # Use thread pool for concurrent upload
        with ThreadPoolExecutor(max_workers=args.workers) as executor:
            # Submit all tasks
            futures = []
            for file_path in files:
                # Local deduplication check
                if args.skip_duplicates:
                    file_hash = calculate_file_hash(file_path)
                    if file_hash in processed_hashes:
                        pbar.update(1)
                        results['duplicate'] += 1
                        continue
                    processed_hashes.add(file_hash)
                
                future = executor.submit(upload_worker, uploader, file_path, metadata, pbar)
                futures.append(future)
            
            # Collect results
            for future in as_completed(futures):
                result = future.result()
                if result['success']:
                    results['success'] += 1
                    if result.get('is_duplicate'):
                        results['duplicate'] += 1
                else:
                    results['failed'] += 1
                    results['errors'].append({
                        'file': result['file'],
                        'error': result['error']
                    })
    
    # Calculate elapsed time
    elapsed_time = time.time() - start_time
    
    # Show results
    print(f"\n{Fore.GREEN}{'='*50}")
    print(f"{Fore.GREEN}Upload completed!")
    print(f"{Fore.GREEN}{'='*50}")
    print(f"  - Total files: {len(files)}")
    print(f"  - Successfully uploaded: {results['success']}")
    print(f"  - Duplicate files: {results['duplicate']}")
    print(f"  - Upload failed: {results['failed']}")
    print(f"  - Total time: {elapsed_time:.1f} seconds")
    print(f"  - Average speed: {len(files) / elapsed_time:.1f} files/second")
    
    # Show error information
    if results['errors']:
        print(f"\n{Fore.RED}Failed file list:")
        for error in results['errors'][:10]:
            print(f"  - {error['file']}: {error['error']}")
        if len(results['errors']) > 10:
            print(f"  ... and {len(results['errors']) - 10} more errors")
    
    # Save upload log
    log_file = f"upload_log_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
    with open(log_file, 'w', encoding='utf-8') as f:
        json.dump({
            'timestamp': datetime.now().isoformat(),
            'directory': str(directory),
            'options': vars(args),
            'results': results
        }, f, ensure_ascii=False, indent=2)
    
    print(f"\n{Fore.CYAN}Detailed log saved to: {log_file}")


if __name__ == '__main__':
    try:
        main()
    except KeyboardInterrupt:
        print(f"\n{Fore.YELLOW}Operation interrupted by user")
        sys.exit(1)
    except Exception as e:
        print(f"\n{Fore.RED}Error: {e}")
        sys.exit(1)
