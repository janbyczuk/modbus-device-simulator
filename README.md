# Modbus Device Simulator

A YAML-configured Modbus server simulator supporting multiple devices and dynamic register sequences.

## Features

- Multiple virtual Modbus devices with unique slave IDs
- YAML-based configuration
- In-memory register storage
- Dynamic sequences triggered on register writes
- Support for holding registers with configurable behaviors

## Configuration Format

The simulator uses YAML configuration files to define virtual Modbus devices. Each device can have multiple holding registers with optional write triggers that execute sequences of actions.

**Note:** The current implementation defaults to slave_id 1 due to limitations in the tokio-modbus library's TCP server API. While the configuration supports multiple devices, the server currently only routes requests to the device with slave_id 1. This is a known limitation that would require modifications to the underlying tokio-modbus library to fully support.

### Basic Example

```yaml
devices:
  - slave_id: 1
    holding_registers:
      - address: 0
        initial_value: 100
      - address: 1
        initial_value: 200
```

### Example with Sequences

```yaml
devices:
  - slave_id: 1
    holding_registers:
      - address: 0
        initial_value: 0
        on_write:
          sequence:
            - action: set
              address: 1
              value: 100
            - action: wait_ms
              duration_ms: 1000
            - action: for_ms
              address: 2
              value: 200
              duration_ms: 2000
      - address: 1
        initial_value: 0
      - address: 2
        initial_value: 0
```

### Configuration Reference

#### Device Configuration

- `slave_id` (required): Modbus slave ID (1-254)
- `holding_registers` (optional): List of holding register configurations

#### Holding Register Configuration

- `address` (required): Register address (0-65535)
- `initial_value` (optional): Initial value of the register (default: 0)
- `on_write` (optional): Sequence to execute when this register is written to

#### Sequence Actions

##### `set` - Set Register Value
Sets a register to a specific value immediately.
```yaml
- action: set
  address: 1
  value: 100
```

##### `for_ms` - Set Value with Duration
Sets a register to a specific value and maintains it for a duration before continuing.
```yaml
- action: for_ms
  address: 1
  value: 200
  duration_ms: 2000
```

##### `wait_ms` - Wait
Waits for a specified duration before continuing to the next step.
```yaml
- action: wait_ms
  duration_ms: 1000
```

## Usage

### Running the Simulator

```bash
mbsim --config device.yaml
```

### Command Line Options

- `--ip` or `-i`: IP address to bind to (default: 127.0.0.1)
- `--port` or `-p`: Port to listen on (default: 5502)
- `--config` or `-c`: Path to YAML configuration file (required)

### Example

```bash
mbsim --ip 0.0.0.0 --port 5502 --config examples/device.yaml
```

## Building

```bash
cargo build --release
```

## Testing

```bash
cargo test
```

