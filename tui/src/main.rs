#![allow(dead_code)]

use sapling::Lang;
use sapling_grammar::tokenizer::ParsedToken;

fn main() {
    parse();
}

fn parse() {
    let json_lang = Lang::load_toml_file("json.toml").unwrap();

    let s = r#"[
    0,
    -3.3149123e+44,
    "HI!",
    {
        "is_awesome": true,
        "message": "This\u0020is a \"newline\": \n"
    }
]"#;

    println!("Parsing:");
    println!("```json");
    println!("{}", s);
    println!("```");

    println!();

    let tree = json_lang.parse_root(s).unwrap();
    dbg!(&tree);

    println!();

    println!("Unparsed tree:");
    println!("```json");
    println!("{}", tree.to_text(json_lang.grammar()).unwrap());
    println!("```");
}

fn tokenize() {
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
            ParsedToken::Stringy(id, content, display_str) => {
                let type_name = json_lang.grammar().type_name(id);
                println!(
                    "node: Stringy({}, {:?}, {})",
                    type_name, content, display_str
                );
            }
        }
        println!("ws  : {:?}", ws);
    }
}
