use sapling_grammar::Grammar;

fn main() {
    let _ = dbg!(Grammar::load_toml_file("json.toml"));
}
