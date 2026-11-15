// Windows resources: the icon and version for the DLL and the program, the manifest for the program.
fn main() {
    if std::env::var("CARGO_CFG_WINDOWS").is_err() {
        return;
    }
    let v = std::env::var("CARGO_PKG_VERSION").unwrap();
    let mut n: Vec<&str> = v.split(['.', '-']).take(3).collect();
    n.resize(4, "0");
    let commas = format!("VERSION_COMMAS={}", n.join(","));
    let dots = format!("VERSION_DOTS=\"{}\"", n.join("."));
    embed_resource::compile_for_everything("res/liyamu.rc", [commas.as_str(), dots.as_str()]);
    embed_resource::compile_for("res/settings.rc", ["liyamu-settings"], embed_resource::NONE);
    println!("cargo:rerun-if-changed=res");
}
