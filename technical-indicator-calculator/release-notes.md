# Technical Indicator Calculator v1.1.0 Release Notes

## Overview
This release introduces a new CLI interface and daemon functionality to the Technical Indicator Calculator, making it easier to manage the indicator calculation service.

## New Features
- **Service-like management commands**: Start, stop, and check status of the indicator calculation service
- **Daemon mode**: Run the calculator in the background with `--detached` flag
- **Robust process management**: Proper handling of process detachment and termination
- **Cross-platform compatibility**: Works on both Unix/Linux and Windows systems
- **Comprehensive status information**: View process details, uptime, and recent logs

## Command Examples
```bash
# Start the calculator in foreground mode
technical-indicator-calculator start

# Start as a background daemon
technical-indicator-calculator start --detached

# Check service status
technical-indicator-calculator status

# Stop the service
technical-indicator-calculator stop

# Continue using other CLI commands
technical-indicator-calculator list
