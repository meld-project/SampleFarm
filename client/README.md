# SampleFarm Client Tools

This directory contains client tools for interacting with the SampleFarm malware analysis platform.

## batch_upload.py

A command-line tool for batch uploading files to SampleFarm with advanced features like concurrent uploads, file filtering, and comprehensive logging.

### Features

- **Batch Upload**: Upload multiple files from a directory in one operation
- **Concurrent Processing**: Multi-threaded uploads for improved performance
- **File Filtering**: Support for file extension filtering and recursive directory scanning
- **Duplicate Detection**: Skip duplicate files based on hash comparison
- **Progress Tracking**: Real-time progress bars and detailed upload statistics
- **Error Handling**: Comprehensive error reporting and retry capabilities
- **Metadata Support**: Rich metadata including labels, source information, and custom fields
- **ZIP Support**: Handle password-protected ZIP archives
- **Dry Run Mode**: Preview files without actual upload
- **Detailed Logging**: JSON-formatted upload logs for audit trails

### Requirements

This tool is designed to run with [uv](https://github.com/astral-sh/uv), a fast Python package manager and project manager.

#### Project Structure

```
client/
├── pyproject.toml          # Project configuration and dependencies
├── .python-version         # Python version specification (3.11)
├── batch_upload.py         # Main upload tool
├── README.md              # This documentation
└── uv.lock                # Dependency lock file (auto-generated)
```

#### Dependencies

The following dependencies are defined in `pyproject.toml`:

- `requests>=2.31.0` - HTTP client library
- `requests-toolbelt>=1.0.0` - Multipart encoding support
- `tqdm>=4.65.0` - Progress bars
- `colorama>=0.4.6` - Cross-platform colored terminal output

### Installation

1. Install uv if you haven't already:
   ```bash
   curl -LsSf https://astral.sh/uv/install.sh | sh
   ```

2. Navigate to the client directory and install dependencies:
   ```bash
   cd client
   uv sync
   ```

   This will:
   - Create a virtual environment automatically (`.venv/`)
   - Install Python 3.11 if not available
   - Install all required dependencies from `pyproject.toml`
   - Generate a lock file (`uv.lock`) for reproducible builds

3. (Optional) Install development dependencies:
   ```bash
   uv sync --extra dev
   ```

   This includes additional tools like `pytest`, `black`, and `flake8` for development.

#### Verify Installation

To verify the installation is working correctly:

```bash
# Check if the tool can be run
uv run batch_upload.py --help

# Test connection to SampleFarm server (requires server to be running)
uv run batch_upload.py /path/to/test/directory --type malicious --dry-run
```

### Usage

#### Basic Syntax

```bash
uv run batch_upload.py <directory_path> --type <sample_type> [options]
```

#### Required Parameters

- `directory_path`: Path to the directory containing files to upload
- `--type/-t`: Sample type, must be either `malicious` or `benign`

#### Optional Parameters

| Parameter | Short | Description | Example |
|-----------|--------|-------------|---------|
| `--source` | `-s` | Sample source identifier | `--source "honeypot"` |
| `--labels` | `-l` | Comma-separated labels | `--labels "trojan,backdoor"` |
| `--recursive` | `-r` | Recursively scan subdirectories | `--recursive` |
| `--extensions` | `-e` | File extensions to include | `--extensions "exe,dll,zip"` |
| `--workers` | `-w` | Number of concurrent threads | `--workers 5` |
| `--api-url` | `-u` | API server URL | `--api-url "http://localhost:8080"` |
| `--passwords` | `-p` | ZIP file passwords | `--passwords "infected,malware"` |
| `--custom-metadata` | `-m` | Custom metadata as JSON | `--custom-metadata '{"analyst":"john"}'` |
| `--dry-run` | | Preview files without upload | `--dry-run` |
| `--skip-duplicates` | | Skip files with duplicate hashes | `--skip-duplicates` |

### Examples

#### Basic Upload

Upload all files in a directory as malicious samples:

```bash
uv run batch_upload.py /path/to/samples --type malicious
```

#### Advanced Upload with Labels

Upload with custom source and labels:

```bash
uv run batch_upload.py /path/to/samples --type malicious --source "honeypot" --labels "trojan,backdoor"
```

#### Recursive Upload with File Filtering

Recursively upload only specific file types:

```bash
uv run batch_upload.py /path/to/samples --type malicious --recursive --extensions "exe,dll,zip"
```

#### High-Performance Upload

Use multiple threads for faster uploads:

```bash
uv run batch_upload.py /path/to/samples --type malicious --workers 10
```

#### Dry Run

Preview what files would be uploaded without actually uploading:

```bash
uv run batch_upload.py /path/to/samples --type malicious --dry-run
```

#### Upload with ZIP Passwords

Handle password-protected ZIP files:

```bash
uv run batch_upload.py /path/to/samples --type malicious --passwords "infected,malware,password123"
```

#### Custom Metadata

Include additional metadata with uploads:

```bash
uv run batch_upload.py /path/to/samples --type malicious --custom-metadata '{"campaign":"apt29","severity":"high"}'
```

### Output and Logging

The tool provides comprehensive output including:

- **Real-time Progress**: Progress bars showing upload status
- **Upload Statistics**: Success/failure counts, duplicate detection, timing information
- **Error Reporting**: Detailed error messages for failed uploads
- **JSON Logs**: Detailed logs saved to `upload_log_YYYYMMDD_HHMMSS.json`

#### Sample Output

```
✓ Successfully connected to SampleFarm server: http://localhost:8080

Scanning directory: /path/to/samples
✓ Found 150 files, total size: 245.67 MB

Upload configuration:
  - Sample type: malicious
  - Sample source: honeypot
  - Labels: trojan, backdoor
  - Concurrent threads: 3

Confirm to start upload? [y/N]: y

Starting upload...
Upload progress: 100%|████████████| 150/150 [02:34<00:00, 0.97files/s]

==================================================
Upload completed!
==================================================
  - Total files: 150
  - Successfully uploaded: 148
  - Duplicate files: 2
  - Upload failed: 0
  - Total time: 154.3 seconds
  - Average speed: 0.97 files/second

Detailed log saved to: upload_log_20241201_143022.json
```

### Configuration

The tool uses the following default configuration:

- **API URL**: `http://localhost:8080`
- **Concurrent Workers**: 3
- **Timeout**: 300 seconds (5 minutes)
- **Max File Size**: 1 GB

These can be overridden using command-line parameters.

### Configuration Files

#### pyproject.toml

The `pyproject.toml` file contains the project configuration:

- **Dependencies**: Core runtime dependencies (requests, tqdm, etc.)
- **Development Dependencies**: Optional tools for development (black, flake8, pytest)
- **Python Version**: Minimum required Python version (3.8.1+)
- **Tool Configuration**: Settings for code formatting and linting tools

The configuration is kept minimal and only includes what's actually used by the script.

### Error Handling

The tool handles various error scenarios:

- **Network Issues**: Automatic retry and timeout handling
- **File Access Errors**: Skip inaccessible files with detailed logging
- **API Errors**: Parse and display server error messages
- **Large Files**: Reject files exceeding size limits
- **Invalid Formats**: Validate metadata and parameters

### Security Considerations

- Files are uploaded using secure multipart/form-data encoding
- ZIP passwords are handled securely and not logged in plain text
- All network communication uses standard HTTP/HTTPS protocols
- File content is streamed to minimize memory usage

### Development

#### Code Formatting and Linting

The project includes development tools configured in `pyproject.toml`:

```bash
# Format code with black
uv run black batch_upload.py

# Check code style with flake8
uv run flake8 batch_upload.py

# Run tests (when available)
uv run pytest
```

#### Adding New Features

1. Install development dependencies: `uv sync --extra dev`
2. Make your changes to `batch_upload.py`
3. Test your changes with `--dry-run` mode
4. Format code with `black` and check with `flake8`
5. Update documentation if needed

### Troubleshooting

#### Common Issues

1. **Installation Issues**
   - Ensure uv is properly installed: `uv --version`
   - Try removing and recreating the virtual environment: `rm -rf .venv uv.lock && uv sync`
   - Check Python version compatibility (requires Python 3.8.1+)

2. **Connection Failed**
   - Verify the SampleFarm server is running
   - Check the API URL is correct
   - Ensure network connectivity
   - Test with: `curl http://localhost:8080/api/status`

3. **Upload Failures**
   - Check file permissions
   - Verify file integrity
   - Review server logs for additional details
   - Try with smaller files first

4. **Performance Issues**
   - Reduce the number of workers (`--workers 1`)
   - Check network bandwidth
   - Consider file sizes and server capacity
   - Monitor system resources

#### Getting Help

For issues and questions:
1. Check the upload logs for detailed error information
2. Review server logs if you have access
3. Use `--dry-run` to test configuration before actual uploads
4. Start with smaller batches to identify problematic files
5. Check uv environment: `uv pip list` to see installed packages

### License

This tool is part of the SampleFarm project. See the main project LICENSE for details.
