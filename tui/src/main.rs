use sapling::Lang;
use sapling_grammar::tokenizer::ParsedToken;

fn main() {
    let json_lang = Lang::load_toml_file("json.toml").unwrap();

    let s = r#" 03  -3.3149123e+44  
]][true, true,
    ,, [false]{: null
"This\u0020is a \"newline\": \n"}}
"#;
    println!("Tokenizing:");
    println!("```json");
    println!("{}", s);
    println!("```");
    println!();

    let (leading_ws, token_iter) = json_lang.tokenize(s);
    println!("ws  : {:?}", leading_ws);
    for tok_result in token_iter {
        let (tok, ws) = tok_result.unwrap();
        match tok {
            ParsedToken::Static(id) => {
                let tok_text = json_lang.grammar().token_text(id);
                println!("node: Static({})", tok_text);
            }
            ParsedToken::Stringy(id, content) => {
                let type_name = json_lang.grammar().type_name(id);
                println!("node: Stringy({}, {:?})", type_name, content);
            }
        }
        println!("ws  : {:?}", ws);
    }
}
