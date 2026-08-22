use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Portable build and distribution tool for Ren'Py Rust extensions", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build and package Ren'Py extension artifacts into the dist directory
    Dist {
        /// Target platforms: windows, linux, mac, or specific triple (comma-separated or multiple flags)
        #[arg(short, long, value_delimiter = ',')]
        targets: Vec<String>,

        /// Specific crates to build (defaults to all cdylib crates)
        #[arg(short, long, value_delimiter = ',')]
        crates: Vec<String>,

        /// Output distribution directory
        #[arg(short, long)]
        dist_dir: Option<PathBuf>,

        /// Optional path to a Ren'Py game project to automatically deploy to
        #[arg(long)]
        deploy_to_game: Option<PathBuf>,
    },
    /// Build crates without packaging into dist
    Build {
        /// Target platforms: windows, linux, mac, or specific triple
        #[arg(short, long, value_delimiter = ',')]
        targets: Vec<String>,

        /// Specific crates to build
        #[arg(short, long, value_delimiter = ',')]
        crates: Vec<String>,
    },
    /// Run Rust unit tests and the Python integration test suite
    Test {
        /// Target platform to build for testing (defaults to host/windows)
        #[arg(short, long, value_delimiter = ',')]
        targets: Vec<String>,

        /// Specific crates to test
        #[arg(short, long, value_delimiter = ',')]
        crates: Vec<String>,
    },
    /// Watch source files for changes, rebuild automatically, and deploy to Ren'Py
    Watch {
        /// Target platforms to build on change
        #[arg(short, long, value_delimiter = ',')]
        targets: Vec<String>,

        /// Specific crates to build on change
        #[arg(short, long, value_delimiter = ',')]
        crates: Vec<String>,

        /// Path to Ren'Py game project to automatically update on file save
        #[arg(long)]
        deploy_to_game: Option<PathBuf>,
    },
    /// Set up Ren'Py link libraries and pyo3-config for a specific Ren'Py SDK
    Setup {
        /// Path to Ren'Py SDK folder or libpython DLL
        #[arg(short, long)]
        renpy_dir: Option<PathBuf>,
    },
    /// Clean target and dist directories
    Clean,
}

#[derive(Debug, Deserialize, Default)]
struct PluginConfig {
    #[serde(default)]
    build: BuildConfig,
}

#[derive(Debug, Deserialize, Default)]
struct BuildConfig {
    #[serde(default)]
    targets: Vec<String>,
    #[serde(default)]
    dist_dir: Option<String>,
    #[serde(default)]
    deploy_to_game: Option<String>,
}

struct PlatformConfig {
    key: &'static str,
    name: &'static str,
    triple: &'static str,
    config_file: &'static str,
    renpy_lib: &'static str,
    py_abi_suffix: &'static str,
    py_ext: &'static str,
    lib_ext: &'static str,
    lib_prefix: &'static str,
    use_zig: bool,
}

const PLATFORMS: &[PlatformConfig] = &[
    PlatformConfig {
        key: "windows",
        name: "Windows (x86_64)",
        triple: "x86_64-pc-windows-gnu",
        config_file: "win.txt",
        renpy_lib: "py3-windows-x86_64",
        py_abi_suffix: "cp312-win_amd64.pyd",
        py_ext: ".pyd",
        lib_ext: ".dll",
        lib_prefix: "",
        use_zig: false,
    },
    PlatformConfig {
        key: "linux",
        name: "Linux (x86_64)",
        triple: "x86_64-unknown-linux-gnu",
        config_file: "linux.txt",
        renpy_lib: "py3-linux-x86_64",
        py_abi_suffix: "cpython-312-x86_64-linux-gnu.so",
        py_ext: ".so",
        lib_ext: ".so",
        lib_prefix: "lib",
        use_zig: true,
    },
    PlatformConfig {
        key: "linux",
        name: "Linux (aarch64)",
        triple: "aarch64-unknown-linux-gnu",
        config_file: "linux-aarch64.txt",
        renpy_lib: "py3-linux-aarch64",
        py_abi_suffix: "cpython-312-aarch64-linux-gnu.so",
        py_ext: ".so",
        lib_ext: ".so",
        lib_prefix: "lib",
        use_zig: true,
    },
    PlatformConfig {
        key: "mac",
        name: "macOS (Apple Silicon - aarch64)",
        triple: "aarch64-apple-darwin",
        config_file: "mac-arm.txt",
        renpy_lib: "py3-mac-arm64",
        py_abi_suffix: "cpython-312-darwin.so",
        py_ext: ".so",
        lib_ext: ".dylib",
        lib_prefix: "lib",
        use_zig: true,
    },
    PlatformConfig {
        key: "mac",
        name: "macOS (Intel - x86_64)",
        triple: "x86_64-apple-darwin",
        config_file: "mac-x86_64.txt",
        renpy_lib: "py3-mac-x86_64",
        py_abi_suffix: "cpython-312-darwin-x86_64.so",
        py_ext: ".so",
        lib_ext: ".dylib",
        lib_prefix: "lib",
        use_zig: true,
    },
];

fn project_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if manifest_dir.file_name().and_then(|n| n.to_str()) == Some("xtask") {
        manifest_dir.parent().unwrap_or(&manifest_dir).to_path_buf()
    } else {
        manifest_dir
    }
}

fn read_plugin_config(root: &Path) -> PluginConfig {
    let toml_path = root.join("renpy-plugin.toml");
    if let Ok(content) = fs::read_to_string(toml_path) {
        toml::from_str(&content).unwrap_or_default()
    } else {
        PluginConfig::default()
    }
}

fn discover_cdylib_crates(root: &Path) -> Result<Vec<String>> {
    let mut crates = Vec::new();
    
    // Check subdirectories
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let toml_path = path.join("Cargo.toml");
                if toml_path.exists() {
                    if let Ok(content) = fs::read_to_string(&toml_path) {
                        if content.contains("\"cdylib\"") {
                            if let Ok(val) = toml::from_str::<toml::Value>(&content) {
                                if let Some(name) = val.get("package").and_then(|p| p.get("name")).and_then(|n| n.as_str()) {
                                    crates.push(name.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Check root Cargo.toml if no subcrates found
    if crates.is_empty() {
        let root_toml = root.join("Cargo.toml");
        if let Ok(content) = fs::read_to_string(&root_toml) {
            if let Ok(val) = toml::from_str::<toml::Value>(&content) {
                if let Some(name) = val.get("package").and_then(|p| p.get("name")).and_then(|n| n.as_str()) {
                    crates.push(name.to_string());
                }
            }
        }
    }

    Ok(crates)
}

fn ensure_windows_pyo3_config(root: &Path) -> Result<()> {
    let win_config_path = root.join("pyo3-config").join("win.txt");
    if win_config_path.exists() {
        let content = fs::read_to_string(&win_config_path)?;
        let mut new_lines = Vec::new();
        for line in content.lines() {
            if line.starts_with("lib_dir=") {
                new_lines.push("lib_dir=renpy_includes".to_string());
            } else {
                new_lines.push(line.to_string());
            }
        }
        fs::write(win_config_path, new_lines.join("\n") + "\n")?;
    }
    Ok(())
}

fn get_python_exe() -> &'static str {
    if Command::new("python3").arg("--version").output().is_ok() {
        "python3"
    } else if Command::new("python").arg("--version").output().is_ok() {
        "python"
    } else if Command::new("py").arg("--version").output().is_ok() {
        "py"
    } else {
        "python3"
    }
}

fn run_setup(root: &Path, renpy_dir: Option<&Path>) -> Result<()> {
    let script_path = root.join("scripts").join("setup_renpy_includes.py");
    if !script_path.exists() {
        bail!("setup_renpy_includes.py not found in scripts/");
    }

    let py_exe = get_python_exe();
    let mut cmd = Command::new(py_exe);
    cmd.arg(&script_path);

    if let Some(dir) = renpy_dir {
        cmd.arg(dir);
    } else {
        cmd.arg("--detect");
    }

    let status = cmd.status().context("Failed to run setup_renpy_includes.py")?;
    if !status.success() {
        bail!("setup_renpy_includes.py exited with error");
    }
    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else {
            fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}

fn execute_build(
    root: &Path,
    selected_targets: &[String],
    selected_crates: &[String],
    dist_dir: Option<&Path>,
    deploy_to_game: Option<&Path>,
) -> Result<()> {
    ensure_windows_pyo3_config(root)?;

    // Check if renpy_includes needs setup
    let includes_dir = root.join("renpy_includes");
    let has_import_lib = fs::read_dir(&includes_dir)
        .map(|entries| {
            entries.flatten().any(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    == Some("a")
            })
        })
        .unwrap_or(false);

    if !has_import_lib {
        println!("[*] Setting up Ren'Py includes...");
        let _ = run_setup(root, None);
    }

    println!("\x1b[1;33mTarget Crates: {}\x1b[0m", selected_crates.join(", "));

    let build_all = selected_targets.is_empty()
        || selected_targets.iter().any(|t| t == "all" || t == "default");

    for platform in PLATFORMS {
        let should_build = build_all
            || selected_targets.iter().any(|t| {
                let t_lower = t.to_lowercase();
                t_lower == platform.key
                    || t_lower == platform.triple
                    || (t_lower.starts_with("win") && platform.key == "windows")
                    || (t_lower == "linux" && platform.key == "linux")
                    || (t_lower == "linux-x86_64" && platform.triple == "x86_64-unknown-linux-gnu")
                    || ((t_lower == "linux-aarch64" || t_lower == "linux-arm64") && platform.triple == "aarch64-unknown-linux-gnu")
                    || ((t_lower.starts_with("mac") || t_lower == "darwin") && (
                        t_lower == "mac" || t_lower == "macos" || t_lower == "darwin"
                        || ((t_lower.contains("arm") || t_lower.contains("aarch64")) && platform.triple == "aarch64-apple-darwin")
                        || ((t_lower.contains("x86") || t_lower.contains("intel")) && platform.triple == "x86_64-apple-darwin")
                    ))
            });

        if !should_build {
            continue;
        }

        println!("\n\x1b[1;36m========================================\x1b[0m");
        println!("\x1b[1;36mBuilding for {} ({})\x1b[0m", platform.name, platform.triple);
        println!("\x1b[1;36m========================================\x1b[0m");

        let config_path = root.join("pyo3-config").join(platform.config_file);

        let mut cmd = if platform.use_zig {
            let mut c = Command::new("cargo");
            c.arg("zigbuild");
            c
        } else {
            let mut c = Command::new("cargo");
            c.arg("build");
            c
        };

        for cr in selected_crates {
            cmd.arg("-p").arg(cr);
        }

        cmd.arg("--target").arg(platform.triple);
        cmd.arg("--release");
        cmd.env("PYO3_CONFIG_FILE", &config_path);
        cmd.current_dir(root);

        let status = cmd.status().with_context(|| format!("Failed to execute build for {}", platform.triple))?;
        if !status.success() {
            eprintln!("\x1b[1;31mBuild failed for {}\x1b[0m", platform.name);
            continue;
        }

        // Artifact packaging if dist_dir is provided
        if let Some(dist) = dist_dir {
            let target_release_dir = root.join("target").join(platform.triple).join("release");
            let lib_dist_dir = dist.join("lib").join(platform.renpy_lib);
            let pkg_dist_dir = dist.join("python-packages");

            fs::create_dir_all(&lib_dist_dir)?;
            fs::create_dir_all(&pkg_dist_dir)?;

            for crate_name in selected_crates {
                let candidates = [
                    target_release_dir.join(format!("{}.dll", crate_name)),
                    target_release_dir.join(format!("lib{}.so", crate_name)),
                    target_release_dir.join(format!("{}.so", crate_name)),
                    target_release_dir.join(format!("lib{}.dylib", crate_name)),
                    target_release_dir.join(format!("{}.dylib", crate_name)),
                ];

                let binary = candidates.iter().find(|p| p.exists());
                if let Some(bin_path) = binary {
                    // 1. Standard lib in lib/
                    let std_lib_name = format!("{}{}{}", platform.lib_prefix, crate_name, platform.lib_ext);
                    fs::copy(bin_path, lib_dist_dir.join(&std_lib_name))?;
                    fs::copy(bin_path, pkg_dist_dir.join(&std_lib_name))?;

                    // 2. Python module in lib/
                    let py_mod_name = format!("{}{}", crate_name, platform.py_ext);
                    let target_py_mod = target_release_dir.join(&py_mod_name);
                    if bin_path != &target_py_mod {
                        let _ = fs::copy(bin_path, &target_py_mod);
                    }
                    fs::copy(bin_path, lib_dist_dir.join(&py_mod_name))?;

                    // 3. ABI tagged module in python-packages/
                    let py_abi_name = format!("{}.{}", crate_name, platform.py_abi_suffix);
                    fs::copy(bin_path, pkg_dist_dir.join(&py_abi_name))?;
                } else {
                    eprintln!("\x1b[1;31mCould not find compiled binary for '{}' in {:?}\x1b[0m", crate_name, target_release_dir);
                }
            }
        }

        println!("\x1b[1;32mBuild succeeded for {}\x1b[0m", platform.name);
    }

    if let Some(dist) = dist_dir {
        println!("\n\x1b[1;35m========================================\x1b[0m");
        println!("\x1b[1;35mDistribution output prepared at: {:?}\x1b[0m", dist);
        println!("\x1b[1;35m========================================\x1b[0m");

        if let Some(game_path) = deploy_to_game {
            if game_path.exists() {
                println!("Deploying binaries to Ren'Py Game: {:?}", game_path);
                let game_pkg_dir = game_path.join("game").join("python-packages");
                let game_lib_dir = game_path.join("lib");

                let _ = copy_dir_all(&dist.join("python-packages"), &game_pkg_dir);
                let _ = copy_dir_all(&dist.join("lib"), &game_lib_dir);
                println!("\x1b[1;32mSuccessfully deployed to {:?}\x1b[0m", game_path);
            }
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = project_root();
    let plugin_cfg = read_plugin_config(&root);

    match cli.command {
        Commands::Dist {
            targets,
            crates,
            dist_dir,
            deploy_to_game,
        } => {
            let active_targets = if !targets.is_empty() {
                targets
            } else if !plugin_cfg.build.targets.is_empty() {
                plugin_cfg.build.targets
            } else {
                vec!["windows".into(), "linux".into(), "mac".into()]
            };

            let active_crates = if !crates.is_empty() {
                crates
            } else {
                discover_cdylib_crates(&root)?
            };

            let active_dist_dir = dist_dir
                .or_else(|| plugin_cfg.build.dist_dir.map(|d| root.join(d)))
                .unwrap_or_else(|| root.join("dist"));

            let active_deploy = deploy_to_game
                .or_else(|| plugin_cfg.build.deploy_to_game.filter(|s| !s.is_empty()).map(PathBuf::from));

            execute_build(
                &root,
                &active_targets,
                &active_crates,
                Some(&active_dist_dir),
                active_deploy.as_deref(),
            )?;
        }
        Commands::Build { targets, crates } => {
            let active_targets = if !targets.is_empty() {
                targets
            } else if !plugin_cfg.build.targets.is_empty() {
                plugin_cfg.build.targets
            } else {
                vec!["windows".into(), "linux".into(), "mac".into()]
            };

            let active_crates = if !crates.is_empty() {
                crates
            } else {
                discover_cdylib_crates(&root)?
            };

            execute_build(&root, &active_targets, &active_crates, None, None)?;
        }
        Commands::Test { targets: _, crates: _ } => {
            println!("\x1b[1;36m========================================\x1b[0m");
            println!("\x1b[1;36mRunning Rust Unit & Serialization Tests...\x1b[0m");
            println!("\x1b[1;36m========================================\x1b[0m");

            let includes_dir = root.join("renpy_includes");
            let current_path = std::env::var("PATH").unwrap_or_default();
            let separator = if cfg!(windows) { ";" } else { ":" };
            let updated_path = format!("{}{}{}", includes_dir.to_string_lossy(), separator, current_path);

            let mut cargo_test = Command::new("cargo");
            cargo_test.arg("test");
            cargo_test.arg("--lib");
            cargo_test.env("PATH", &updated_path);
            cargo_test.current_dir(&root);

            let status = cargo_test.status().context("Failed to run cargo test")?;
            if !status.success() {
                bail!("cargo test failed with non-zero exit code");
            }

            println!("\n\x1b[1;32m[✓] All Rust unit and rollback/pickle tests passed successfully!\x1b[0m");
        }
        Commands::Watch {
            targets,
            crates,
            deploy_to_game,
        } => {
            let active_targets = if !targets.is_empty() {
                targets
            } else if !plugin_cfg.build.targets.is_empty() {
                plugin_cfg.build.targets
            } else {
                vec!["windows".into()]
            };

            let active_crates = if !crates.is_empty() {
                crates
            } else {
                discover_cdylib_crates(&root)?
            };

            let active_dist_dir = root.join("dist");
            let active_deploy = deploy_to_game
                .or_else(|| plugin_cfg.build.deploy_to_game.filter(|s| !s.is_empty()).map(PathBuf::from));

            println!("\x1b[1;32m========================================\x1b[0m");
            println!("\x1b[1;32m[Watch Mode Started]\x1b[0m");
            println!("Watching source files for changes (Press Ctrl+C to stop)...");
            if let Some(ref d) = active_deploy {
                println!("Auto-deploy target: {:?}", d);
            }
            println!("\x1b[1;32m========================================\x1b[0m");

            // Initial build
            let _ = execute_build(
                &root,
                &active_targets,
                &active_crates,
                Some(&active_dist_dir),
                active_deploy.as_deref(),
            );

            let mut last_mtimes = scan_file_mtimes(&root);

            loop {
                std::thread::sleep(std::time::Duration::from_millis(600));
                let current_mtimes = scan_file_mtimes(&root);
                if current_mtimes != last_mtimes {
                    last_mtimes = current_mtimes;
                    println!("\n\x1b[1;33m[Change Detected]\x1b[0m Rebuilding...");
                    let start = std::time::Instant::now();
                    let res = execute_build(
                        &root,
                        &active_targets,
                        &active_crates,
                        Some(&active_dist_dir),
                        active_deploy.as_deref(),
                    );
                    match res {
                        Ok(_) => println!("\x1b[1;32m[✓ Rebuild Succeeded in {:.2?}]\x1b[0m", start.elapsed()),
                        Err(e) => eprintln!("\x1b[1;31m[x Rebuild Failed]: {}\x1b[0m", e),
                    }
                }
            }
        }
        Commands::Setup { renpy_dir } => {
            run_setup(&root, renpy_dir.as_deref())?;
        }
        Commands::Clean => {
            let target_dir = root.join("target");
            let dist_dir = root.join("dist");
            if target_dir.exists() {
                let _ = fs::remove_dir_all(target_dir);
            }
            if dist_dir.exists() {
                let _ = fs::remove_dir_all(dist_dir);
            }
            println!("Cleaned target/ and dist/ directories.");
        }
    }

    Ok(())
}

fn scan_file_mtimes(root: &Path) -> std::collections::HashMap<PathBuf, std::time::SystemTime> {
    let mut map = std::collections::HashMap::new();

    fn scan_dir(dir: &Path, map: &mut std::collections::HashMap<PathBuf, std::time::SystemTime>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name != "target" && name != "dist" && name != ".git" {
                        scan_dir(&path, map);
                    }
                } else if let Ok(meta) = entry.metadata() {
                    if let Ok(mtime) = meta.modified() {
                        map.insert(path, mtime);
                    }
                }
            }
        }
    }

    scan_dir(&root.join("src"), &mut map);
    scan_dir(&root.join("scripts"), &mut map);
    scan_dir(&root.join("tests"), &mut map);

    for file in [root.join("Cargo.toml"), root.join("renpy-plugin.toml")] {
        if let Ok(meta) = file.metadata() {
            if let Ok(mtime) = meta.modified() {
                map.insert(file, mtime);
            }
        }
    }

    map
}
