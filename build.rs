fn main() {
    let shaders = ["fs.glsl", "vs.glsl", "blue.glsl"];
    for shader in &shaders {
        println!("cargo: rerun-if-changed=src/{}", shader);
    }
}
