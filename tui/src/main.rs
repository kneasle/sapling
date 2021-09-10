use sapling::Lang;

fn main() {
    let _ = dbg!(Lang::load_toml_file("json.toml"));
}
