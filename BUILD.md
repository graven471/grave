# Build System

This project uses **Cargo + `xtask`** to build the Rust kernel, prepare a UEFI-bootable ISO, and run it in QEMU.

The build system targets **x86-64 bare metal** and uses Limine as the bootloader.

## Requirements

On Arch Linux, install the required tools:

```bash
sudo pacman -S --needed \
    rustup \
    xorriso \
    qemu-system-x86 \
    edk2-ovmf \
    git \
    make
```

Ensure the required Rust target is installed:

```bash
rustup target add x86_64-unknown-none
```

`git` and `make` are only needed if you use them elsewhere; the current `xtask` design does not clone or build Limine.

## Project Layout

```text
grave/
├── .cargo/
│   └── config.toml
├── boot/
│   ├── efi/
│   │   └── BOOTX64.EFI
│   ├── limine/
│   │   └── limine.conf
│   ├── limine-uefi-cd.bin
│   └── firmware/
│       ├── OVMF_CODE.fd
│       └── OVMF_VARS.fd
├── linker/
│   └── x86_64.ld
├── src/
│   └── main.rs
├── xtask/
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
├── Cargo.toml
└── BUILD.md
```

Generated files are stored in `build/` and Cargo's `target/` directory.

## Everyday Development

The normal development command is:

```bash
cargo xtask run
```

This runs the complete pipeline:

1. Compile the kernel with Cargo.
2. Verify the required Limine files exist.
3. Prepare the ISO staging directory.
4. Copy the kernel, boot configuration, and UEFI bootloader files.
5. Generate `build/kernel.iso` using `xorriso`.
6. Launch QEMU with OVMF UEFI firmware.

After editing kernel code, run `cargo xtask run` again.

## Available Commands

| Command | Description |
|---|---|
| `cargo xtask build` | Compile the kernel ELF. |
| `cargo xtask limine` | Verify the locally supplied Limine files. |
| `cargo xtask iso` | Build the kernel and generate the ISO. |
| `cargo xtask run` | Build, generate the ISO, and boot QEMU. |
| `cargo xtask debug` | Build and boot QEMU paused for GDB debugging. |
| `cargo xtask clean` | Remove generated `build/` artifacts. |

You do not need to run `build`, `iso`, and `run` separately. The `run` command invokes the preceding steps automatically.

## Kernel Build

The kernel is built for:

```text
x86_64-unknown-none
```

The expected debug ELF is:

```text
target/x86_64-unknown-none/debug/grave
```

The `.cargo/config.toml` file configures the target and linker script. The `xtask` itself runs on the host, Arch Linux, because it uses the standard library and invokes host tools.

## Limine Files

Limine is managed locally. The build system does not clone the Limine repository or download bootloader files.

Required files:

- `boot/efi/BOOTX64.EFI` — UEFI bootloader executable.
- `boot/limine-uefi-cd.bin` — UEFI CD boot image.
- `boot/limine/limine.conf` — boot configuration.

Obtain these files from the **same official Limine binary release**:

https://github.com/limine-bootloader/limine/releases

The Rust `limine` Cargo crate is separate: it provides Rust-side protocol definitions and does not replace the bootloader binaries.

The optional splash image, if used, is:

```text
boot/limine/splash.png
```

## UEFI Firmware

QEMU uses OVMF firmware from `edk2-ovmf`.

On Arch Linux, the firmware files are typically located at:

```text
/usr/share/edk2/x64/OVMF_CODE.4m.fd
/usr/share/edk2/x64/OVMF_VARS.4m.fd
```

The project uses these local copies:

```text
boot/firmware/OVMF_CODE.fd
boot/firmware/OVMF_VARS.fd
```

- `OVMF_CODE.fd` contains the UEFI firmware code.
- `OVMF_VARS.fd` stores writable UEFI NVRAM variables.

The code image is used read-only by QEMU. The variables image is writable and can retain firmware state between runs. Preserve a clean copy if you need to reset UEFI settings.

## QEMU Configuration

The current configuration uses:

- Machine: `q35`
- CPU: `max`
- CPUs: `1`
- Memory: `8192M`
- Firmware: OVMF UEFI
- Serial output: standard I/O
- Automatic reboot: disabled
- Automatic shutdown: disabled

QEMU starts from the generated ISO.

## Debugging with GDB

Start QEMU paused:

```bash
cargo xtask debug
```

In another terminal, launch GDB:

```bash
gdb target/x86_64-unknown-none/debug/grave
```

Connect to QEMU's GDB server:

```gdb
target remote :1234
```

Then use standard GDB commands, such as:

```gdb
info registers
break _start
continue
```

The `-s` option opens the GDB server on port 1234, while `-S` pauses the guest CPU until GDB resumes it.

## Cleaning Build Artifacts

Run:

```bash
cargo xtask clean
```

This removes the generated `build/` directory. It does not remove Cargo's `target/` directory or the firmware files under `boot/firmware/`.

To clean Cargo artifacts as well:

```bash
cargo clean
```

## Troubleshooting

**`Required Limine file missing`**

Check the paths listed in the Limine Files section. Ensure the UEFI executable and CD image belong to the same release.

**`xorriso` fails**

Check that `xorriso` is installed and that the ISO-generation options match the Limine release you're using.

**QEMU cannot find firmware**

Check that both `boot/firmware/OVMF_CODE.fd` and `boot/firmware/OVMF_VARS.fd` exist.

**Kernel does not boot**

A successful compilation and ISO generation do not guarantee a working kernel. Check the Limine configuration, ELF entry point, linker script, boot protocol requirements, and serial output.

---

**Typical development loop:**

```text
Edit kernel code
      ↓
cargo xtask run
      ↓
Kernel builds → ISO generated → QEMU boots
      ↓
Inspect output and repeat
```
