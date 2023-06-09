use xdiff::{LoadConfig, RequestConfig};

fn main() {
    // include_str! 从指定文件的内容嵌入到编译时生成的可执行文件中，返回 &'static str
    let config = RequestConfig::load_yaml("fixtures/xreq_test.yml").unwrap();
    println!("【 config 】==> {:#?}", config);
}
