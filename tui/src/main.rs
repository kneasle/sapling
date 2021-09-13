use sapling::Lang;

fn main() {
    let json_lang = Lang::load_toml_file("json.toml").unwrap();

    let (leading_whitespace, token_iter) =
        json_lang.token_iter("  ]][true, true,,, [false]{: null}}\n");
    let tokens: Vec<_> = token_iter
        .map(|tok_result| {
            tok_result
                .map(|(tok_id, whitespace)| (json_lang.grammar().token_text(tok_id), whitespace))
        })
        .collect::<Result<_, _>>()
        .unwrap();
    println!("{:?}", (leading_whitespace, tokens));
}
