use voice_control::parser::IsParser;
fn main() {
    println!("{}", voice_control::parser::roundy::parser().describe());
    voice_control::voice_control(voice_control::parser::roundy::parser);
}
