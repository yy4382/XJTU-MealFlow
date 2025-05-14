// my_cli_server/build.rs
use std::path::Path;
use std::process::Command;

fn main() {
    // 1. 获取当前 profile (debug/release)
    // let profile = env::var("PROFILE").unwrap();

    // 2. 定义前端目录
    let frontend_dir = Path::new("frontend");

    // 3. 提示 Cargo 如果前端文件改变则重新运行 build.rs
    // 更精细的控制可以只监控 src 和 public
    println!("cargo:rerun-if-changed={}/src", frontend_dir.display());
    println!("cargo:rerun-if-changed={}/public", frontend_dir.display());
    println!(
        "cargo:rerun-if-changed={}/package.json",
        frontend_dir.display()
    );
    println!(
        "cargo:rerun-if-changed={}/pnpm-lock.yaml",
        frontend_dir.display()
    ); // or yarn.lock

    // 4. 安装前端依赖 (可选，但推荐在 CI 或初次构建时)
    // 注意：在本地开发时，你可能更倾向于手动管理 node_modules
    // 如果 package-lock.json 或 yarn.lock 有变化，或者 node_modules 不存在，则运行 install
    // 为简单起见，这里可以先跳过自动 npm install，假设开发者会手动运行
    if !frontend_dir.join("node_modules").exists() {
        println!(
            "cargo:warning=Running 'pnpm install' in {}...",
            frontend_dir.display()
        );
        let install_status = Command::new("pnpm") // 或者 "yarn"
            .current_dir(&frontend_dir)
            .arg("install")
            .status()
            .expect("Failed to execute pnpm install. Is Node.js/pnpm installed and in PATH?");

        if !install_status.success() {
            panic!("pnpm install failed");
        }
    }

    // 5. 构建前端项目
    println!(
        "cargo:warning=Building frontend app in {}...",
        frontend_dir.display()
    );

    let pnpm_build_cmd = if cfg!(windows) { "pnpm.cmd" } else { "pnpm" };

    let build_status = Command::new(pnpm_build_cmd)
        .current_dir(&frontend_dir)
        .args(["run", "build"]) 
        .status()
        .expect("Failed to execute pnpm build. Is Node.js/pnpm installed and in PATH, and is there a 'build' script in package.json?");

    if !build_status.success() {
        panic!("Frontend build failed (e.g., pnpm run build). Check frontend project for errors.");
    }

    println!("cargo:warning=Frontend build successful.");
}
