use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

struct Config {
    root: PathBuf,
    build: PathBuf,
    iso_root: PathBuf,
    iso: PathBuf,
    firmware: PathBuf,
    kernel: PathBuf,
}

impl Config {
    fn new() -> Result<Self> {
        let root = env::current_dir()?;
        let build = root.join("build");

        Ok(Self {
            kernel: root.join("target/x86_64-unknown-none/debug/grave"),
            iso_root: build.join("iso"),
            iso: build.join("kernel.iso"),
            firmware: root.join("boot/firmware"),
            build,
            root,
        })
    }

    fn boot_dir(&self) -> PathBuf {
        self.iso_root.join("boot")
    }

    fn efi_dir(&self) -> PathBuf {
        self.iso_root.join("EFI/BOOT")
    }

    fn limine_efi(&self) -> PathBuf {
        self.root.join("boot/efi/BOOTX64.EFI")
    }

    fn limine_cd_image(&self) -> PathBuf {
        self.root.join("boot/limine-uefi-cd.bin")
    }

    fn limine_config(&self) -> PathBuf {
        self.root.join("boot/limine/limine.conf")
    }

    fn splash(&self) -> PathBuf {
        self.root.join("boot/limine/splash.png")
    }

    fn firmware_code(&self) -> PathBuf {
        self.firmware.join("OVMF_CODE.fd")
    }

    fn firmware_vars(&self) -> PathBuf {
        self.firmware.join("OVMF_VARS.fd")
    }
}

fn run(program: &str, args: &[&str], cwd: Option<&Path>) -> Result<()> {
    println!("+ {} {}", program, args.join(" "));

    let mut command = Command::new(program);
    command.args(args);

    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }

    let status = command.status()?;
    ensure_success(status, program)
}

fn ensure_success(status: ExitStatus, program: &str) -> Result<()> {
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} exited with status {status}").into())
    }
}

fn copy_file(source: &Path, destination: &Path) -> Result<()> {
    if !source.is_file() {
        return Err(format!("Required file does not exist: {}", source.display()).into());
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(source, destination)?;
    println!("  {} -> {}", source.display(), destination.display());
    Ok(())
}

fn find_first_existing(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|path| path.is_file()).cloned()
}

fn system_ovmf_file(names: &[&str]) -> Result<PathBuf> {
    let mut candidates = Vec::new();

    for name in names {
        candidates.push(PathBuf::from("/usr/share/edk2/x64").join(name));
        candidates.push(PathBuf::from("/usr/share/OVMF").join(name));
    }

    find_first_existing(&candidates).ok_or_else(|| {
        format!(
            "Could not find any of these firmware files: {}. Install edk2-ovmf.",
            names.join(", ")
        )
        .into()
    })
}

fn build_kernel(config: &Config) -> Result<()> {
    run(
        "cargo",
        &["build", "--target", "x86_64-unknown-none"],
        Some(&config.root),
    )?;

    if !config.kernel.is_file() {
        return Err(format!(
            "Kernel ELF not found at {}. Check the package name in Cargo.toml.",
            config.kernel.display()
        )
        .into());
    }

    Ok(())
}

fn ensure_limine(config: &Config) -> Result<()> {
    for path in [&config.limine_efi(), &config.limine_cd_image()] {
        if !path.is_file() {
            return Err(format!("Required Limine file missing: {}", path.display()).into());
        }
    }

    Ok(())
}

fn stage_iso(config: &Config) -> Result<()> {
    if config.iso_root.exists() {
        fs::remove_dir_all(&config.iso_root)?;
    }

    fs::create_dir_all(config.boot_dir())?;
    fs::create_dir_all(config.efi_dir())?;

    copy_file(&config.kernel, &config.boot_dir().join("kernel.elf"))?;
    copy_file(
        &config.limine_config(),
        &config.boot_dir().join("limine.conf"),
    )?;

    copy_file(&config.limine_efi(), &config.efi_dir().join("BOOTX64.EFI"))?;

    copy_file(
        &config.limine_cd_image(),
        &config.iso_root.join("limine-uefi-cd.bin"),
    )?;

    if config.splash().is_file() {
        copy_file(&config.splash(), &config.boot_dir().join("splash.png"))?;
    }

    Ok(())
}

fn make_iso(config: &Config) -> Result<()> {
    build_kernel(config)?;
    ensure_limine(config)?;
    stage_iso(config)?;

    fs::create_dir_all(&config.build)?;

    let iso = config.iso.to_string_lossy().into_owned();
    let root = config.iso_root.to_string_lossy().into_owned();

    run(
        "xorriso",
        &[
            "-as",
            "mkisofs",
            "-R",
            "-r",
            "--efi-boot",
            "limine-uefi-cd.bin",
            "--efi-boot-part",
            "--efi-boot-image",
            "-o",
            &iso,
            &root,
        ],
        Some(&config.root),
    )?;

    println!("ISO created: {}", config.iso.display());
    Ok(())
}

fn prepare_firmware(config: &Config) -> Result<()> {
    fs::create_dir_all(&config.firmware)?;

    let code = config.firmware_code();
    let vars = config.firmware_vars();

    if !code.is_file() {
        let source = system_ovmf_file(&["OVMF_CODE.4m.fd", "OVMF_CODE.fd"])?;
        copy_file(&source, &code)?;
    }

    if !vars.is_file() {
        let source = system_ovmf_file(&["OVMF_VARS.4m.fd", "OVMF_VARS.fd"])?;
        copy_file(&source, &vars)?;
    }

    Ok(())
}

fn qemu(config: &Config, debug: bool) -> Result<()> {
    make_iso(config)?;
    prepare_firmware(config)?;

    let code = config.firmware_code().to_string_lossy().into_owned();
    let vars = config.firmware_vars().to_string_lossy().into_owned();
    let iso = config.iso.to_string_lossy().into_owned();

    let mut args = vec![
        "-machine".to_string(),
        "q35".to_string(),
        "-cpu".to_string(),
        "max".to_string(),
        "-smp".to_string(),
        "1".to_string(),
        "-m".to_string(),
        "8192M".to_string(),
        "-drive".to_string(),
        format!("if=pflash,format=raw,readonly=on,file={code}"),
        "-drive".to_string(),
        format!("if=pflash,format=raw,file={vars}"),
        "-cdrom".to_string(),
        iso,
        "-display".to_string(),
        "gtk".to_string(),
        "-serial".to_string(),
        "stdio".to_string(),
        "-global".to_string(),
        "isa-debugcon.iobase=0x402".to_string(),
        "-monitor".to_string(),
        "none".to_string(),
        // "-d".to_string(),
        // "int,cpu_reset".to_string(),
        // "-D".to_string(),
        // "/tmp/qemu.log".to_string(),
        "-no-reboot".to_string(),
        "-no-shutdown".to_string(),
    ];

    if debug {
        args.push("-s".to_string());
        args.push("-S".to_string());
    }

    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run("qemu-system-x86_64", &refs, Some(&config.root))
}

fn clean(config: &Config) -> Result<()> {
    if config.build.exists() {
        fs::remove_dir_all(&config.build)?;
    }

    // Keep the user's firmware copies, matching the original Makefile's
    // behavior: `clean` removes build output, not boot/firmware.
    println!("Removed {}", config.build.display());
    Ok(())
}

fn usage() {
    eprintln!(
        "Usage: cargo xtask <command>\n\
         Commands:\n\
           build   Compile the kernel ELF\n\
           limine  Fetch/build the pinned Limine UEFI bootloader\n\
           iso     Build the UEFI bootable ISO\n\
           run     Boot the ISO in QEMU\n\
           debug   Boot QEMU paused with a GDB server on port 1234\n\
           clean   Remove generated build artifacts"
    );
}

fn main() -> Result<()> {
    let config = Config::new()?;
    let command = env::args().nth(1).unwrap_or_default();

    match command.as_str() {
        "build" => build_kernel(&config),
        "limine" => ensure_limine(&config),
        "iso" => make_iso(&config),
        "run" => qemu(&config, false),
        "debug" => qemu(&config, true),
        "clean" => clean(&config),
        _ => {
            usage();
            Err(format!("Unknown command: {command}").into())
        }
    }
}
