use std::process::Command;
use std::str;

fn main() {
    // Only re-run if .git directory changes, though simpler is just to run always
    // println!("cargo:rerun-if-changed=.git/HEAD"); // This is not reliable for all git changes
    // It's usually fine to just run it on every build.

    let output = Command::new("git")
        .args(["describe", "--tags", "--always", "--broken"])
        .current_dir(env!("CARGO_MANIFEST_DIR")) // Run git in the project root
        .output()
        .expect("Failed to execute git command");

    let git_tag = if output.status.success() {
        str::from_utf8(&output.stdout)
            .expect("Invalid UTF-8 from git")
            .trim()
            .to_string()
    } else {
        // Fallback for when git isn't available or fails
        eprintln!("WARNING: git describe failed: {:?}", output.stderr);
        "unknown".to_string()
    };

    println!("cargo:rustc-env=GIT_TAG={git_tag}");
}
