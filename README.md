# Christmas Ornament Firmware

A battery-powered Christmas ornament with dual-battery backup, written in Rust using [Embassy](https://embassy.dev/) on STM32L031G6U6S.

This project also serves as an example of using Nix flakes with embedded Rust and embassy-rs.

## Hardware

The ornament uses an ultra-low-power STM32L031 MCU running from two coin cell batteries with automatic failover. The MCU controls 6 LEDs (3 red, 3 green) via D flip-flops, alternating between red and green patterns. The system spends most of its time in STOP mode, waking only when the RTC timer expires or when the battery voltage drops below threshold.

- **MCU**: STM32L031G6U6S (Cortex-M0+)
- **Power**: Dual coin cells with MIC94050 load switches
- **LED Control**: 2× SN74LVC1G74 D flip-flops
- **LEDs**: 6 total (3 red + 3 green) at ~255µA each
- **RTC**: 32.768 kHz crystal for timekeeping in STOP mode

See [schematic.pdf](schematic.pdf) for the complete circuit design.

## Design

### Power Management

The system uses two independent power rails selected by load switches. A Programmable Voltage Detector (PVD) monitors VDD at 2.7V and triggers an interrupt when the voltage drops. The interrupt handler performs a make-before-break switch to the backup battery, preventing power loss during the transition.

The PVD is connected to EXTI line 16, which can wake the MCU from STOP mode. This allows battery monitoring with zero active polling current.

### LED Control

Rather than driving the LEDs directly from the MCU, the design uses D flip-flops to maintain LED state while the MCU is in STOP mode. The MCU wakes periodically, clocks new data into the flip-flops, and immediately returns to sleep. The flip-flops continue driving the LEDs with no further MCU involvement.

This approach dramatically reduces power consumption compared to keeping the MCU awake for LED control.

### Low Power Operation

The firmware configures the MSI oscillator at 66 kHz and relies on Embassy's async executor to automatically enter STOP mode when no tasks are runnable. The RTC continues running from the external 32.768 kHz crystal, providing accurate timing even in deep sleep.

Typical power profile:
- STOP mode: ~1µA (MCU) + ~510µA (LEDs when on)
- Active time: ~1ms per LED update cycle

## Building and Flashing

### Prerequisites

Install Nix with flakes:
```bash
curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install
```

Hardware: ST-Link/V2 or compatible SWD debugger

### Build

```bash
nix build
# Binary at: result/bin/christmas-rs

# Or with cargo:
cargo build --release --target thumbv6m-none-eabi
```

### Flash

The project uses cargo's runner configuration for flashing:

```bash
cargo run --release
```

This automatically invokes `probe-rs run --chip STM32L031G6` after building (configured in `.cargo/config.toml`).

### Development

```bash
nix develop  # Enter development shell with all tools

# Available:
cargo build --release
cargo run --release
cargo doc --open --no-deps
nix fmt  # Format all code (Rust, Nix, etc.)
```

For automatic environment activation, use direnv:
```bash
echo "use flake" > .envrc
direnv allow
```

## Debug Mode

The firmware includes a `debug-mode` feature that enables detailed logging over RTT and uses a faster clock (2 MHz) to maintain a stable debug connection.

To build and run with debug mode:

```bash
cargo run --release --features debug-mode
```

Debug mode provides detailed logs for initialization, LED state transitions, PVD events, and battery switching.

## Using Nix with Embedded Rust

This project demonstrates a complete Nix flake setup for embedded Rust development. The key components:

**Toolchain Management**

The `rust-toolchain.toml` file specifies nightly Rust with the `thumbv6m-none-eabi` target. The Nix flake uses `rust-overlay` to read this file, ensuring the exact same toolchain in development, CI, and builds:

```nix
rustToolchain = rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
```

No need for rustup - Nix handles everything.

**Build Configuration**

The package derivation requires embedded-specific configuration:

```nix
rustPlatform.buildRustPackage {
  auditable = false;  # Required for embedded targets (rust-lld)
  doCheck = false;    # No std on embedded targets

  buildPhase = ''
    cargo build --release --target thumbv6m-none-eabi
  '';
}
```

**Development Shell**

The devShell uses `inputsFrom` to inherit all dependencies from the package, avoiding duplication:

```nix
mkShell {
  inputsFrom = [ pkgs.christmas ];
  nativeBuildInputs = [ /* dev-only tools */ ];
}
```

When you update package dependencies, the devShell automatically inherits those changes.

**Formatting**

The formatter is configured via treefmt-nix with `rustfmt` enabled, so `nix fmt` formats all code (Rust, Nix, Markdown, etc.) consistently.

**CI**

GitHub Actions runs:
- `nix flake check` - Validates flake structure
- `nix build` - Builds firmware
- `cargo fmt --check` - Enforces formatting
- `cargo clippy -- -D warnings` - Lints code
- `cargo doc` - Builds documentation

All cargo commands run in `nix develop` for consistency with local development.

## Project Structure

```
christmas-rs/
├── .github/workflows/ci.yml    # CI configuration
├── src/
│   ├── main.rs                 # Application entry point
│   ├── power.rs                # Dual-battery management
│   ├── string_controller.rs    # LED control via flip-flops
│   └── hardware.rs             # Pin mappings
├── nix/
│   ├── packages/christmas.nix  # Build derivation
│   ├── devShells.nix           # Development environment
│   ├── overlays.nix            # Rust overlay composition
│   └── formatter.nix           # treefmt configuration
├── flake.nix                   # Flake entry point
├── rust-toolchain.toml         # Rust toolchain specification
└── .cargo/config.toml          # Build target and probe-rs runner
```

## Documentation

The codebase includes comprehensive rustdoc comments. Generate and view:

```bash
cargo doc --open --no-deps
```

## License

MIT

## References

- [Embassy Documentation](https://embassy.dev/)
- [Nix Flakes](https://nixos.wiki/wiki/Flakes)
- [rust-overlay](https://github.com/oxalica/rust-overlay)
- [probe-rs](https://probe.rs/)
- [STM32L0 Reference Manual](https://www.st.com/resource/en/reference_manual/rm0377-ultralowpower-stm32l0x1-advanced-armbased-32bit-mcus-stmicroelectronics.pdf)
